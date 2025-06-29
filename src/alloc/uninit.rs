use crate::alloc::SliceDst;
use core::{
    mem::{ManuallyDrop, MaybeUninit},
    ptr,
};

#[repr(transparent)]
pub struct UnsizedMaybeUninit<T: ?Sized>(T);

impl<T: SliceDst + ?Sized> UnsizedMaybeUninit<T> {
    fn as_mut_ptr(&mut self) -> *mut T {
        let (ptr, size) = ptr::from_mut(self).to_raw_parts();

        ptr::from_raw_parts_mut(ptr, size)
    }
}
