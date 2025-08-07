use core::{alloc::Layout, cell::SyncUnsafeCell, ffi::CStr, fmt::Debug, slice};

use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes};

use crate::alloc::{CStyleAllocator, MemoryMapped};

static PAGE_SIZE: SyncUnsafeCell<usize> = SyncUnsafeCell::new(0);

pub fn get_page_size() -> usize {
  // safety: so always valid because reading from a static, only written to on startup
  unsafe { PAGE_SIZE.get().read() }
}

#[non_exhaustive]
#[derive(TryFromBytes, Immutable, PartialEq, Debug)]
#[repr(usize)]
enum AuxiliaryVecKey {
  EndOfVector = libc::AT_NULL as usize,
  IgnoredField = libc::AT_IGNORE as usize,
  ProgramFileDescriptor = libc::AT_EXECFD as usize,
  ProgramHeaders = libc::AT_PHDR as usize,
  ProgramHeaderEntrySize = libc::AT_PHENT as usize,
  ProgramHeaderCount = libc::AT_PHNUM as usize,
  PageSize = libc::AT_PAGESZ as usize,
  InterpreterBaseAddress = libc::AT_BASE as usize,
  Flags = libc::AT_FLAGS as usize,
  Entrypoint = libc::AT_ENTRY as usize,
  IsNonElf = libc::AT_NOTELF as usize,
  RealUserId = libc::AT_UID as usize,
  EffectiveUserId = libc::AT_EUID as usize,
  RealGroupId = libc::AT_GID as usize,
  EffectiveGroupId = libc::AT_EGID as usize,
  TimesSyscallClockFrequency = libc::AT_CLKTCK as usize,
}

#[derive(TryFromBytes, KnownLayout, Immutable, Debug)]
#[repr(C)]
struct AuxiliaryVecEntry {
  key: AuxiliaryVecKey,
  value: usize,
}

#[derive(IntoBytes, Immutable, Debug)]
#[repr(C)]
struct AuxiliaryVecEntryRaw {
  key: usize,
  value: usize,
}

impl AuxiliaryVecEntryRaw {
  pub fn try_convert(&'static self) -> Result<&'static AuxiliaryVecEntry, &'static Self> {
    AuxiliaryVecEntry::try_ref_from_bytes(self.as_bytes())
      .ok()
      .ok_or(self)
  }
}

struct AuxiliaryVec {
  ptr: *const AuxiliaryVecEntryRaw,
}

impl Iterator for AuxiliaryVec {
  type Item = &'static AuxiliaryVecEntryRaw;
  fn next(&mut self) -> Option<Self::Item> {
    // safety: current value always points to a valid entry, up to and including the end of the vector
    let mut value = unsafe { self.ptr.as_ref().unwrap() };

    while value.key == AuxiliaryVecKey::IgnoredField as _ {
      // safety: next value is known to be valid
      self.ptr = unsafe { self.ptr.add(1) };
      // safety: ptr is valid to read from
      value = unsafe { self.ptr.as_ref().unwrap() };
    }

    if value.key == AuxiliaryVecKey::EndOfVector as _ {
      return None;
    }

    // safety: next value is known to be valid
    self.ptr = unsafe { self.ptr.add(1) };

    Some(value)
  }
}

pub fn handle_args(argc: isize, argv: *const *const u8) {
  linux_handle_args(argc, argv);
}

static ARGS: SyncUnsafeCell<&'static [&'static CStr]> = SyncUnsafeCell::new(&[]);

#[linkage = "weak"]
#[unsafe(no_mangle)]
#[inline(never)]
extern "C" fn linux_handle_args(argc: isize, argv: *const *const u8) {
  unsafe {
    let argc: usize = argc
      .try_into()
      .expect("argc was negative or too large (why??)");
    let args = slice::from_raw_parts::<'static>(argv, argc as usize)
      .iter()
      .map(|ptr| CStr::from_ptr(ptr.cast()));
    let args_len = args
      .clone()
      .map(|arg| size_of::<&CStr>() + arg.to_bytes_with_nul().len())
      .sum();
    let arg_alloc = MemoryMapped
      .alloc(Layout::array::<u8>(args_len).unwrap())
      .unwrap();
    let mut value_offset = argc * size_of::<&CStr>();
    for (index, arg) in args.enumerate() {
      let bytes = arg.to_bytes_with_nul();
      let new_ptr = arg_alloc.cast::<u8>().add(value_offset).as_ptr();
      new_ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());

      arg_alloc
        .add(index)
        .cast::<&CStr>()
        .write(CStr::from_ptr(new_ptr as _));
      value_offset += bytes.len();
    }
    ARGS.get().write(slice::from_raw_parts(
      arg_alloc.cast::<&CStr>().as_ptr(),
      argc,
    ));

    let mut envp = argv.add(argc).add(1);
    while !envp.read().is_null() {
      envp = envp.add(1);
    }

    let auxv = envp.add(1).cast::<AuxiliaryVecEntryRaw>();
    for ele in (AuxiliaryVec { ptr: auxv }) {
      match ele.try_convert() {
        Ok(value) => {
          // println!("handled: {value:X?}");
          if value.key == AuxiliaryVecKey::PageSize {
            PAGE_SIZE.get().write(value.value);
          }
        }
        Err(_value) => {
          // println!("unknown: {value:X?}")
        }
      }
    }
  };
}
