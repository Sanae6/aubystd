use core::ops::DerefMut;

// pub trait ArrayAllocator<Element> {
//   type AllocateError;
//   type Handle<'allocator>: ArrayHandle<Element> + 'allocator
//   where
//     Self: 'allocator;

//   async fn allocate<'allocator>(&'allocator self, element_count: usize) -> Result<Self::Handle<'allocator>, Self::AllocateError>;
// }

// free on drop !!!
// pub trait ArrayHandle<Element>: DerefMut<Target = [Element]> {
//   fn free(self) -> 
// } 
