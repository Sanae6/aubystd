use core::{arch::naked_asm, cell::UnsafeCell, mem::offset_of};
use syscalls::Sysno;

use crate::{
  alloc::strategy::Arc,
  platform::linux::thread::{CloneArgs, ThreadRegion},
  println,
  thread::ThreadParker,
};

struct ThreadStack {
  thread_arc: UnsafeCell<Arc<'static, ThreadRegion>>,
  stack_size: usize,
  thread_func_ptr: *mut (),
  thread_func_size: usize,
  func: unsafe fn(&dyn ThreadParker, *mut ()),
}

pub fn prepare_stack<F: FnOnce(&dyn ThreadParker) + Send + Sync + 'static>(
  clone_args: &mut CloneArgs,
  thread_region: Arc<'static, ThreadRegion>,
  func: F,
) {
  let stack_size = clone_args.stack_size as usize;
  let stack_top = clone_args.stack_lowest_byte_ptr.value();
  let stack_bottom = unsafe { stack_top.add(stack_size) };

  let thread_stack = unsafe { stack_bottom.cast::<ThreadStack>().sub(1) };

  let thread_func = unsafe { thread_stack.cast::<F>().sub(1) };
  unsafe { thread_func.write(func) };

  // safety: stack always valid
  unsafe {
    thread_stack.write(ThreadStack {
      thread_arc: thread_region.into(),
      stack_size,
      thread_func_ptr: thread_func.cast(),
      thread_func_size: size_of::<F>(),
      func: |parker, ptr| {
        ptr.cast::<F>().read()(parker)
      },
    })
  };

  // println!("stack top: {stack_top:?}, stack bottom: {stack_bottom:?}");
  //println!("thread stack value: {thread_func:?}");
}

#[unsafe(naked)]
pub unsafe extern "sysv64" fn spawn_thread(clone_args: *mut CloneArgs) -> usize {
  naked_asm!(
    "push rbp",
    "mov rbp, rsp",
    "mov rax, {clone3}", // clone3
    "mov rsi, {clone_args_size}", // sizeof<CloneArgs>()
    "syscall",
    "cmp rax, 0",
    "jnz 2f", // handle main thread and error
    // we're on the second thread!
    "mov rdi, rsp",
    "sub rdi, {stack_struct_size}",
    "mov rsp, rdi",
    "mov r12, rdi",
    "sub rsp, [rdi + {thread_func_size_offset}]",
    "call {handle_thread}",
    // done, prepare to exit
    "mov rsi, [r12 + {stack_size_offset}]",
    "sub rdi, {stack_struct_size}",
    "add rdi, rsi",
    "mov rax, {munmap}",
    "syscall",
    "jnz 3f",
    // perform exit syscall
    "mov rax, {exit}", // exit
    "syscall",
    "3:",
    "hlt", // unreachable!()
    // exit on main thread (success and failure)
    "2: pop rbp",
    "ret",
    clone3 = const Sysno::clone3 as usize,
    clone_args_size = const size_of::<CloneArgs>(),
    stack_struct_size = const size_of::<ThreadStack>(),
    stack_size_offset = const offset_of!(ThreadStack, stack_size),
    thread_func_size_offset = const offset_of!(ThreadStack, thread_func_size),
    handle_thread = sym handle_thread,
    munmap = const Sysno::munmap as usize,
    exit = const Sysno::exit as usize,
  )
}

unsafe extern "sysv64" fn handle_thread(thread_stack: *mut ThreadStack) {
  let thread_stack = unsafe { &*thread_stack };
  let thread_region = unsafe { thread_stack.thread_arc.get().read() };

  unsafe { (thread_stack.func)(thread_region.as_ref(), thread_stack.thread_func_ptr) }
}
