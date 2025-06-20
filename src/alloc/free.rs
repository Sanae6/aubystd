use core::{
  alloc::Layout, marker::{PhantomInvariantLifetime, variance}, ptr::{NonNull, null_mut}
};

pub struct FreeVtable<'allocator> {
  free_fn: unsafe fn(context: *mut (), allocation: *mut (), layout: Layout),
  context: *mut (),
  lifetime: PhantomInvariantLifetime<'allocator>,
}

impl<'allocator> FreeVtable<'allocator> {
  pub fn new<C: ?Sized>(
    free_fn: unsafe fn(context: *mut (), allocation: *mut (), layout: Layout),
    context: *mut C,
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

  /// Safety: There cannot be any held references to the allocation data during this function call.
  /// This is normally called from a function like [`drop`].
  ///
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
  ///     unsafe { (&raw mut (*self.0.as_ptr()).free_vtable).read().free(self.0, Layout::new::<ExampleData<T>>()) };
  ///   }
  /// }
  /// ```
  pub unsafe fn free<A: ?Sized>(self, allocation: NonNull<A>, layout: Layout) {
    // Safety: The context and caller-provided allocation passed to the free function are expected to be what the function expects.
    unsafe { (self.free_fn)(self.context, allocation.as_ptr() as *mut _, layout) }
  }
}
