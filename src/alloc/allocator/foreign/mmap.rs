use core::{alloc::Layout, ptr};

use crate::{num::align_up_checked, platform::active::rt::get_page_size, println};
use bitflags::bitflags;
use syscalls::{Sysno, syscall};

use crate::alloc::{CStyleAllocator, OutOfMemory};
bitflags! {
  pub struct MemoryMapProtection: u32 {
    const NONE = libc::PROT_NONE as u32;
    const READ = libc::PROT_READ as u32;
    const WRITE = libc::PROT_WRITE as u32;
    const READ_WRITE = (libc::PROT_READ | libc::PROT_WRITE) as u32;
    const EXEC = libc::PROT_EXEC as u32;
  }

  pub struct MemoryMapFlags: u32 {
    const PRIVATE = libc::MAP_PRIVATE as u32;
    const ANONYMOUS = libc::MAP_ANONYMOUS as u32;
    const GROWS_DOWN = libc::MAP_GROWSDOWN as u32;
    const STACK = libc::MAP_STACK as u32;
  }
}

#[derive(Default)]
pub struct MemoryMapped;

impl MemoryMapped {
  // todo: macos mmap implementation
  // todo: windows mmap implementation
  #[cfg(target_os = "linux")]
  pub unsafe fn map_without_file(
    &self,
    address: *mut u8,
    size: usize,
    prot: MemoryMapProtection,
    flags: MemoryMapFlags,
  ) -> Result<*mut u8, syscalls::Errno> {
    use syscalls::Sysno;

    let address = unsafe {
      syscall!(
        Sysno::mmap,
        address,
        size,
        prot.bits(),
        flags.bits(),
        -1i32,
        0u32
      )
    }.map(|address| address as *mut u8);

    address
  }

  pub unsafe fn unmap(&self, address: *mut u8, size: usize) -> Result<(), syscalls::Errno> {
    unsafe { syscall!(Sysno::munmap, address, size) }?;

    Ok(())
  }
}

unsafe impl CStyleAllocator for MemoryMapped {
  fn alloc(&self, layout: Layout) -> Result<ptr::NonNull<u8>, OutOfMemory> {
    let padded_layout = layout
      .align_to(0x1000)
      .map_err(|_| OutOfMemory)?
      .pad_to_align();
    let total_size = if padded_layout.align() > 0x1000 {
      panic!("too aligned! {:?}", padded_layout);
    } else {
      padded_layout.size()
    };

    let address = unsafe {
      self.map_without_file(
        ptr::null_mut(),
        total_size,
        MemoryMapProtection::READ_WRITE,
        MemoryMapFlags::PRIVATE | MemoryMapFlags::ANONYMOUS,
      )
    }
    .map_err(|error| {
      println!("{error:?}");
      OutOfMemory
    })?;

    ptr::NonNull::new(address as *mut u8).ok_or(OutOfMemory)
  }

  unsafe fn free(&self, ptr: ptr::NonNull<u8>, layout: Layout) {
    let page_size = get_page_size();
    unsafe {
      self
        .unmap(
          ptr.as_ptr().map_addr(|addr| addr & !page_size),
          align_up_checked(layout.size(), page_size).expect("page size could not be aligned"),
        )
        .expect("could not unmap memory")
    };
  }
}
