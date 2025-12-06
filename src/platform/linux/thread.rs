mod x86_64;
// todo: arm64, arm32

use core::ptr;

use crate::{
  alloc::{
    ForeignAllocator, MemoryMapped,
    mmap::{MemoryMapFlags, MemoryMapProtection},
    strategy::{Arc, ArcStrategy},
  },
  platform::linux::{FileDescriptor, ProcessId, U64Ptr, sync::thread_parker::LinuxThreadParker},
  thread::{ThreadHandle, ThreadParker, ThreadUnresponsive, Threading},
};

pub struct LinuxThreading;

#[repr(C)]
struct CloneArgs {
  flags: u64,
  pid_file_descriptor_ptr: U64Ptr<FileDescriptor>,
  child_thread_id_ptr: U64Ptr<ProcessId>,
  parent_thread_id_ptr: U64Ptr<ProcessId>,
  exit_signal: u64,
  stack_lowest_byte_ptr: U64Ptr<u8>,
  stack_size: u64,
  thread_local_storage: U64Ptr<u8>,
  set_thread_id_ptr: U64Ptr<[ProcessId]>,
  set_thread_id_size: u64,
  control_group: u64,
}

static ALLOCATOR: ForeignAllocator<MemoryMapped> = ForeignAllocator::new(MemoryMapped);

struct ThreadRegion {
  thread_id: i32,
  thread_parker: LinuxThreadParker,
}

impl Threading for LinuxThreading {
  async fn spawn<F: FnOnce(&dyn ThreadParker) + Send + Sync + 'static>(
    &'static self,
    stack_size: usize,
    func: F,
  ) -> Arc<'static, dyn ThreadHandle> {
    let stack_ptr = unsafe {
      MemoryMapped.map_without_file(
        ptr::null_mut(),
        stack_size,
        MemoryMapProtection::READ_WRITE,
        MemoryMapFlags::PRIVATE | MemoryMapFlags::ANONYMOUS | MemoryMapFlags::STACK,
      )
    }
    .expect("failed to allocate memory for stack");

    let thread_region: Arc<'static, ThreadRegion> = ALLOCATOR
      .take::<ArcStrategy>(ThreadRegion {
        thread_id: 0,
        thread_parker: LinuxThreadParker::new(),
      })
      .await
      .unwrap();

    let mut clone_args = CloneArgs {
      flags: (libc::CLONE_VM
        | libc::CLONE_THREAD
        | libc::CLONE_FILES
        | libc::CLONE_SIGHAND
        //| libc::CLONE_CHILD_SETTID
        //| libc::CLONE_SETTLS
        | 0) as _,
      pid_file_descriptor_ptr: U64Ptr::null(),
      child_thread_id_ptr: U64Ptr::null(),
      parent_thread_id_ptr: U64Ptr::null(),
      exit_signal: 0,
      stack_lowest_byte_ptr: U64Ptr::new(stack_ptr),
      stack_size: stack_size as u64,
      thread_local_storage: U64Ptr::null(),
      set_thread_id_ptr: U64Ptr::null(),
      set_thread_id_size: 0,
      control_group: 0,
    };

    // safety: the stack is valid for all accesses
    unsafe {
      x86_64::prepare_stack(&mut clone_args, thread_region.clone(), func)
    }

    // safety: clone_args is validly initialized
    let _thread_id = unsafe { x86_64::spawn_thread(&raw mut clone_args) };

    thread_region
  }
}

impl ThreadHandle for ThreadRegion {
  fn id(&self) -> usize {
    self.thread_id as _
  }

  fn unpark(&self) -> Result<(), ThreadUnresponsive> {
    self.thread_parker.wake();
    Ok(())
  }

  fn join(&self) -> Result<(), ThreadUnresponsive> {
    self.thread_parker.sleep();
    Ok(())
  }
}

impl ThreadParker for ThreadRegion {
  fn park(&self) {
    self.thread_parker.sleep();
  }
}
