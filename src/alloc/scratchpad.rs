#![allow(unused)]

use core::{any::Any, marker::CoercePointee, mem::MaybeUninit, ops::DerefMut};

// pub trait Allocator<T> {
//   type AllocateError;
//   type Handle: AllocationHandle<MaybeUninit<T>>;

//   fn allocate(&self) -> impl Future<Output = Result<Self::Handle, Self::AllocateError>> + Send;
// }

// pub trait AllocationHandle<T>: DerefMut<Target = T> {
//   type FreeError;

//   fn free(self) -> impl Future<Output = Result<(), Self::FreeError>>;
// }

// pub trait ArrayAllocator<E> {
//   type AllocateError;
//   type Entry: ArrayAllocationHandle<E>;

//   fn allocate(&self, length: usize) -> impl Future<Output = Result<Self::Entry, Self::AllocateError>> + Send;
// }

// pub trait ArrayAllocationHandle<T>: DerefMut<Target = [T]> {
//   type FreeError;

//   fn free(self) -> impl Future<Output = Result<(), Self::FreeError>>;
// }

// struct AnyAlloc;
// #[derive(CoercePointee)]
// #[repr(transparent)]
// struct AnyAllocHandle<T: ?Sized>(core::ptr::Unique<T>);

// impl Allocator<u32> for AnyAlloc {
//   type AllocateError = ();

//   type Handle = AnyAllocHandle<u32>;

//   fn allocate(&self) -> impl Future<Output = Result<Self::Entry, Self::AllocateError>> + Send {
//     todo!()
//   }
// }

// impl<T: Any> AllocationHandle for AnyAlloc {
//   type FreeError;

//   fn free(self) -> impl Future<Output = Result<(), Self::FreeError>> {
//     todo!()
//   }
// }

// pub struct AH;
// impl AllocationHandle<[u8]> for AH {
//   type FreeError = ();

//   fn free(self) -> impl Future<Output = Result<(), Self::FreeError>> {
//     async move { Ok(()) }
//   }
// }

// impl Deref for AH {
//   type Target = [u8];

//   fn deref(&self) -> &Self::Target {
//     todo!()
//   }
// }

// impl DerefMut for AH {
//   fn deref_mut(&mut self) -> &mut Self::Target {
//     todo!()
//   }
// }
