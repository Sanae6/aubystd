use core::{
  alloc::Layout, marker::variance, ptr::{NonNull, null_mut}
};

use crate::alloc::strategy::StrategyVariance;

pub struct FreeVtable<'a> {
  free_fn: unsafe fn(context: *const (), allocation: *const (), layout: Layout),
  context: *const (),
  lifetime: StrategyVariance<'a>,
}

impl<'a> FreeVtable<'a> {
  pub fn new<C: ?Sized>(
    free_fn: unsafe fn(context: *const (), allocation: *const (), layout: Layout),
    context: *const C,
  ) -> Self {
    Self {
      free_fn: free_fn,
      context: context as _,
      lifetime: variance(),
    }
  }

  pub const fn new_empty() -> Self {
    Self {
      free_fn: |_, _, _| {},
      context: null_mut(),
      lifetime: variance(),
    }
  }

  /// Frees the allication related to this
  /// ## Example
  /// An example of a [StrategyHandle](super::strategy::StrategyHandle)-like type holding allocation data and its respective
  ///
  /// [`drop`]: core::ops::Drop::drop
  /// ```
  /// use core::{alloc::Layout, ptr::NonNull};
  /// use aubystd::alloc::FreeVtable;
  ///
  /// struct ExampleData<'a, T> {
  ///   free_vtable: FreeVtable<'a>,
  ///   value: T,
  /// }
  ///
  /// struct ExampleHandle<'a, T>(NonNull<ExampleData<'a, T>>);
  ///
  /// impl<'a, T> Drop for ExampleHandle<'a, T> {
  ///   fn drop(&mut self) {
  ///     // Here we read the value in order to get a copy and call free.
  ///     // The reason we don't get a reference to free_vtable is because the allocator expects
  ///     // full ownership over the data in the allocator, which the vtable resides in.
  ///     unsafe {
  ///       let free_vtable = (&raw mut (*self.0.as_ptr()).free_vtable).read();
  ///       free_vtable.free(self.0, Layout::new::<ExampleData<T>>());
  ///     }
  ///   }
  /// }
  ///
  /// ```
  /// ## Safety
  /// - `allocation` must be the pointer created alongside this vtable.
  /// - `layout` must be the layout of the allocation used in its creation.
  /// - There cannot be any held references to the allocation data during or after this function call.
  /// This is normally called from a function like [`drop`].
  pub unsafe fn free<A: ?Sized>(self, allocation: NonNull<A>, layout: Layout) {
    // Safety: The context and caller-provided allocation passed to the free function are expected to be what the function expects.
    unsafe { (self.free_fn)(self.context, allocation.as_ptr() as *mut _, layout) }
  }
}
