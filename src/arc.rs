use std::{
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{self, AtomicUsize, Ordering},
};

pub struct Arc<T> {
    ptr: NonNull<ArcInner<T>>,         // covariant over T and not null
    phantom: PhantomData<ArcInner<T>>, // own a ArcInner<T> value (which owns T)
}

struct ArcInner<T> {
    rc: AtomicUsize,
    data: T,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        let boxed = Box::new(ArcInner {
            rc: AtomicUsize::new(1),
            data,
        });

        Arc {
            ptr: NonNull::new(Box::into_raw(boxed)).unwrap(), // unwrap is safe - Box::into_raw is guaranteed non-null
            phantom: PhantomData,
        }
    }
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Arc<T> {
        let inner = unsafe { self.ptr.as_ref() };
        // Relaxed ordering on `inner.rc` is fine because we're not accessing `inner.data`
        // Knowledge of the original reference prevents other threads from deleting the object
        let old_rc = inner.rc.fetch_add(1, Ordering::Relaxed);

        if old_rc >= isize::MAX as usize {
            std::process::abort();
        }

        Self {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        let old_rc = inner.rc.fetch_sub(1, Ordering::Release);

        if old_rc != 1 {
            return;
        }

        // Fence is required to prevent reordering of the use and deletion of the data
        atomic::fence(Ordering::Acquire);

        // Safe as we have the last pointer to the `ArcInner` and the pointer is valid
        unsafe {
            drop(Box::from_raw(self.ptr.as_ptr()));
        }
    }
}
