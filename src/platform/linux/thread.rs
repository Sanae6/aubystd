mod x86_64;

use core::{arch::asm, ptr};

use syscalls::{Errno, Sysno, syscall};

use crate::{
  alloc::{
    ForeignAllocator, MemoryMapped,
    mmap::{MemoryMapFlags, MemoryMapProtection},
    strategy::{Arc, ArcStrategy},
  },
  platform::linux::{FileDescriptor, ProcessId, U64Ptr, sync::thread_parker::LinuxThreadParker},
  println,
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

fn free_stack() {}

unsafe impl Threading for LinuxThreading {
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
        MemoryMapFlags::PRIVATE | MemoryMapFlags::ANONYMOUS,
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
        | libc::CLONE_FS
        | libc::CLONE_SIGHAND
        | libc::CLONE_CHILD_SETTID
        | libc::CLONE_SETTLS
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
    x86_64::prepare_stack(&mut clone_args, thread_region.clone(), func);

    let ret = unsafe { x86_64::spawn_thread(&raw mut clone_args) };
    if let Err(errno) = Errno::from_ret(ret) {
      let region = clone_args
        .thread_local_storage
        .value()
        .cast::<Arc<ThreadRegion>>();

      // safety: the region arc stored on the allocated stack is valid to access and drop since
      // the new thread failed to start.
      unsafe {
        region.drop_in_place();
      }
      panic!("failed to create thread: {errno:?}")
    }

    // let thread_id = {
    //   let mut ret: usize;
    //   // safety: arguments to clone are valid, safe to use.
    //   // wasn't able to use syscalls to perform the operation because dev profile won't optimize the
    //   // call tree away, and accessing the stack is ub on the new thread.
    //   unsafe {
    //     asm!(
    //       "syscall",
    //       inlateout("rax") Sysno::clone3 as usize => ret,
    //       in("rdi") &raw mut clone_args,
    //       in("rsi") size_of::<CloneArgs>(),
    //       out("rcx") _, // rcx is used to store old rip
    //       out("r11") _, // r11 is used to store old rflags
    //       options(nostack, preserves_flags)
    //     );
    //   }
    // };

    // if thread_id == 0 {
    //   // safety: we can't access the old stack, and we shall not
    //   unsafe {
    //     let new_tr_ptr: *mut Arc<'static, ThreadRegion>;
    //     let thread_region = unsafe {
    //       asm!("mov {}, fs:0", out(reg) new_tr_ptr);
    //       new_tr_ptr.read()
    //     };
    //     func(thread_region.as_ref());
    //     unsafe {
    //       syscall!(Sysno::exit, 0).unwrap();
    //     }
    //     println!("not exited...");
    //     unreachable!();
    //   }
    // }

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
}

impl ThreadParker for ThreadRegion {
  fn park(&self) {
    self.thread_parker.sleep();
  }
}
