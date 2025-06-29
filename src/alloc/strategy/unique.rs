use core::{
    alloc::Layout,
    borrow::{Borrow, BorrowMut},
    fmt::{self, Debug, Display},
    marker::{CoercePointee, PhantomCovariantLifetime},
    mem::MaybeUninit,
    ops::{Deref, DerefMut, DerefPure},
    pin::Pin,
    ptr,
};

use crate::alloc::{UnsizedMaybeUninit, strategy::PinStrategyHandle};

use super::{
    FreeVtable, SliceDst, Strategy, StrategyDataPtr, StrategyHandle, UninitStrategyHandleExt,
};

#[derive(Default)]
pub struct UniqueStrategy;
pub const UNIQUE: UniqueStrategy = UniqueStrategy;

#[doc(hidden)]
#[derive(SliceDst)]
#[repr(C)]
pub struct UniqueData<'a, T: ?Sized> {
    free_vtable: FreeVtable<'a>,
    value: T,
}

impl Strategy for UniqueStrategy {
    type SizedData<'allocator, T: 'allocator> = UniqueData<'allocator, T>;
    type SliceData<'allocator, T: ?Sized + 'allocator> = UniqueData<'allocator, T>;
    type SizedHandle<'allocator, T: 'allocator> = Unique<'allocator, T>;
    type SliceHandle<'allocator, T: ?Sized + 'allocator> = Unique<'allocator, T>;
    type UninitSizedHandle<'allocator, T: 'allocator> = Unique<'allocator, MaybeUninit<T>>;
    type UninitSliceHandle<'allocator, T: ?Sized + 'allocator> =
        Unique<'allocator, UnsizedMaybeUninit<T>>;

    fn initialize_data_sized<'allocator, T: 'allocator>(
        free_vtable: FreeVtable<'allocator>,
        data_ptr: *mut UniqueData<'allocator, T>,
    ) {
        unsafe {
            (&raw mut (*data_ptr).free_vtable).write(free_vtable);
        }
    }

    fn initialize_data_slice<'allocator, T: ?Sized + 'allocator>(
        free_vtable: FreeVtable<'allocator>,
        data_ptr: *mut Self::SliceData<'allocator, T>,
    ) {
        unsafe {
            (&raw mut (*data_ptr).free_vtable).write(free_vtable);
        }
    }

    fn construct_handle_sized<'allocator, T: 'allocator>(
        ptr: ptr::NonNull<UniqueData<'allocator, MaybeUninit<T>>>,
    ) -> Self::UninitSizedHandle<'allocator, T> {
        Unique(ptr, Default::default())
    }

    fn construct_handle_slice<'allocator, T: ?Sized + 'allocator>(
        ptr: ptr::NonNull<UniqueData<'allocator, UnsizedMaybeUninit<T>>>,
    ) -> Self::UninitSliceHandle<'allocator, T> {
        Unique(ptr, Default::default())
    }
}

#[derive(CoercePointee)]
#[repr(transparent)]
pub struct Unique<'allocator, T: ?Sized + 'allocator>(
    ptr::NonNull<UniqueData<'allocator, T>>,
    PhantomCovariantLifetime<'allocator>,
);

impl<'allocator, T> Unique<'allocator, T> {
    pub fn into_inner(self) -> T {
        unsafe {
            let data = self.0.read();
            let layout = Layout::for_value_raw(self.0.as_ptr() as *const _);
            (&raw mut (*self.0.as_ptr()).free_vtable)
                .read()
                .free(self.0, layout);
            data.value
        }
    }
}

impl<'allocator, T: ?Sized> Unique<'allocator, T> {
    pub fn into_pin(unique: Self) -> Pin<Self> {
        // safety comment from Box::into_pin
        // It's not possible to move or replace the insides of a `Pin<Unique<T>>`
        // when `T: !Unpin`, so it's safe to pin it directly without any
        // additional requirements.
        unsafe { Pin::new_unchecked(unique) }
    }
}

impl<'allocator, T: ?Sized> StrategyHandle<T> for Unique<'allocator, T> {
    type Cast<'cast, U: ?Sized + 'cast> = Unique<'cast, U>;

    fn as_ptr(&self) -> *mut T {
        unsafe { (&raw mut (*self.0.as_ptr()).value) }
    }

    fn into_strategy_data_ptr(self) -> StrategyDataPtr<T> {
        let strategy_data_ptr = StrategyDataPtr {
            strategy_data_ptr: unsafe { ptr::NonNull::new_unchecked(self.0.as_ptr() as *mut _) },
            value: self.as_ptr(),
        };

        core::mem::forget(self);

        strategy_data_ptr
    }

    unsafe fn from_strategy_data_ptr(
        StrategyDataPtr {
            strategy_data_ptr: start_ptr,
            value: value_ptr,
        }: StrategyDataPtr<T>,
    ) -> Self {
        let ptr = ptr::from_raw_parts_mut(start_ptr.as_ptr(), value_ptr.to_raw_parts().1);
        Unique(
            unsafe { ptr::NonNull::new_unchecked(ptr) },
            Default::default(),
        )
    }
}

impl<'a, T: ?Sized> Borrow<T> for Unique<'a, T> {
    fn borrow(&self) -> &T {
        unsafe { &self.0.as_ref().value }
    }
}

impl<'a, T: ?Sized> BorrowMut<T> for Unique<'a, T> {
    fn borrow_mut(&mut self) -> &mut T {
        unsafe { &mut self.0.as_mut().value }
    }
}

impl<'a, T: ?Sized> AsRef<T> for Unique<'a, T> {
    fn as_ref(&self) -> &T {
        self.borrow()
    }
}

impl<'a, T: ?Sized> AsMut<T> for Unique<'a, T> {
    fn as_mut(&mut self) -> &mut T {
        self.borrow_mut()
    }
}

impl<'a, T: ?Sized> Deref for Unique<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().value }
    }
}

impl<'a, T: ?Sized> DerefMut for Unique<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.0.as_mut().value }
    }
}

unsafe impl<'a, T: ?Sized> DerefPure for Unique<'a, T> {}

impl<'a, T: ?Sized> Drop for Unique<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.0.drop_in_place();
            let layout = Layout::for_value_raw(self.0.as_ptr() as *const _);
            (&raw mut (*self.0.as_ptr()).free_vtable)
                .read()
                .free(self.0, layout);
        }
    }
}

impl<'a, T: Debug + ?Sized> Debug for Unique<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'a, T: Display + ?Sized> Display for Unique<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'a, T> UninitStrategyHandleExt<MaybeUninit<T>> for Unique<'a, MaybeUninit<T>> {
    type Init = Unique<'a, T>;

    unsafe fn assume_init(self) -> Self::Init {
        let (strategy_data, value) = Self::into_strategy_data_ptr(self).into_pair();
        let (ptr, size) = value.to_raw_parts();
        let value = ptr::from_raw_parts_mut(ptr, size);
        unsafe { Unique::from_strategy_data_ptr(StrategyDataPtr::from_pair(strategy_data, value)) }
    }
}

impl<'a, T: ?Sized> UninitStrategyHandleExt<UnsizedMaybeUninit<T>>
    for Unique<'a, UnsizedMaybeUninit<T>>
{
    type Init = Unique<'a, T>;

    unsafe fn assume_init(self) -> Self::Init {
        let StrategyDataPtr {
            strategy_data_ptr: strategy_data,
            value,
        } = Self::into_strategy_data_ptr(self);
        let (ptr, size) = value.to_raw_parts();
        let value = ptr::from_raw_parts_mut(ptr, size);
        unsafe {
            Unique::from_strategy_data_ptr(StrategyDataPtr {
                strategy_data_ptr: strategy_data,
                value,
            })
        }
    }
}

impl<'a, T: ?Sized> Unpin for Unique<'a, T> {}

impl<'a, T: ?Sized> PinStrategyHandle<T> for Unique<'a, T> {
    fn into_pin(self) -> Pin<Self> {
        // See alloc::boxed::Box::into_pin
        unsafe { Pin::new_unchecked(self) }
    }
}
