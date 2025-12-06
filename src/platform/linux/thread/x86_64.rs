use core::{arch::naked_asm, cell::UnsafeCell, mem::offset_of};
use syscalls::Sysno;

use crate::{
  alloc::{strategy::Arc, MemoryMapped},
  platform::linux::{rt, thread::{CloneArgs, ThreadRegion}},
  thread::ThreadParker,
};

struct ThreadStack {
  thread_arc: UnsafeCell<Arc<'static, ThreadRegion>>,
  stack_size: usize,
  thread_func_ptr: *mut (),
  thread_func_size: usize,
  func: unsafe fn(&dyn ThreadParker, *mut ()),
}

/// Safety: the stack in `clone_args` must be valid
pub unsafe fn prepare_stack<F: FnOnce(&dyn ThreadParker) + Send + Sync + 'static>(
  clone_args: &mut CloneArgs,
  thread_region: Arc<'static, ThreadRegion>,
  func: F,
) {
  let stack_size = clone_args.stack_size as usize;
  let stack_top = clone_args.stack_lowest_byte_ptr.value();

  assert!(
    size_of::<F>() + size_of::<ThreadStack>() < stack_size,
    "thread function stack usage is too large to spawn thread"
  );

  // the stack is asserted to be valid
  unsafe {
    let stack_bottom = stack_top.add(stack_size);
    println!("stack top: {stack_top:?}, stack bottom: {stack_bottom:?}");

    let thread_stack = stack_bottom.cast::<ThreadStack>().sub(1);

    let thread_func = thread_stack.cast::<F>().sub(1);
    thread_func.write(func);

    thread_stack.write(ThreadStack {
      thread_arc: thread_region.into(),
      stack_size,
      thread_func_ptr: thread_func.cast(),
      thread_func_size: size_of::<F>(),
      func: |parker, ptr| ptr.cast::<F>().read()(parker),
    })
  }
}

// safety: the stack must be valid to be called
unsafe fn failure_case(clone_args: &mut CloneArgs) {
  let stack_size = clone_args.stack_size as usize;
  let stack_top = clone_args.stack_lowest_byte_ptr.value();

  // safety: the stack is valid
  unsafe {
    let stack_bottom = stack_top.add(stack_size);

    let thread_stack = stack_bottom.cast::<ThreadStack>().sub(1);
    (*thread_stack).thread_arc.get().drop_in_place();

    MemoryMapped.unmap(stack_top, stack_size).unwrap();
  }

  panic!("thread crashed");
}

#[unsafe(naked)]
pub unsafe extern "sysv64" fn spawn_thread(clone_args: *mut CloneArgs) -> usize {
  naked_asm!(
    "push rbp",
    "mov rbp, rsp",
    // rdi is already clone_args because of extern "sysv64"
    "mov rax, {clone3}", // clone3
    "mov rsi, {clone_args_size}", // sizeof<CloneArgs>()
    "syscall",
    "cmp rax, 0",
    "jl {failure_case}", // handle error
    "jnz 2f", // handle main thread
    // we're on the second thread!
    "mov rdi, rsp",
    "mov r13, rsp",
    "sub rdi, {stack_struct_size}",
    "mov rsp, rdi",
    "mov r12, rdi",
    "sub rsp, [rdi + {thread_func_size_offset}]",
    "call {handle_thread}",
    // done, prepare to exit
    "mov r12, [r12 + {stack_size_offset}]",
    //"mov rdi, r13",
    //"mov rax, {gettid}",
    //"syscall",
    //"mov r12, rsi",
    //"mov r13, rdi",
    //"mov rdi, rax",
    //"mov rax, {tkill}",
    //"mov r14, [rip+ {pid_static}]",
    //"mov esi, 5", // sigtrap
    //"syscall",
    //"jmp 3f",
    "mov rax, {munmap}",
    "mov rdi, r13",
    "mov rsi, r12",
    "syscall",
    "cmp rax, 0",
    "jne 3f",
    // perform exit syscall
    "mov rax, {exit}", // exit
    "mov rdi, 0",
    "syscall",
    "3:",
    "hlt", // unreachable!()
    // exit on main thread
    "2: pop rbp",
    "ret",
    clone3 = const Sysno::clone3 as usize,
    failure_case = sym failure_case,
    clone_args_size = const size_of::<CloneArgs>(),
    stack_struct_size = const size_of::<ThreadStack>(),
    stack_size_offset = const offset_of!(ThreadStack, stack_size),
    thread_func_size_offset = const offset_of!(ThreadStack, thread_func_size),
    handle_thread = sym handle_thread,
    munmap = const Sysno::munmap as usize,
    exit = const Sysno::exit as usize,
    //tkill = const Sysno::tkill as usize,
    //pid_static = sym rt::PID,
    //gettid = const Sysno::gettid as usize,
  )
}

/// Safety: `thread_stack` and its values must be valid
unsafe extern "sysv64" fn handle_thread(thread_stack: *mut ThreadStack) {
  let (thread_stack, thread_region) = unsafe {
    let thread_stack = &*thread_stack;
    (thread_stack, thread_stack.thread_arc.get().read())
  };

  unsafe { (thread_stack.func)(thread_region.as_ref(), thread_stack.thread_func_ptr) }
}
