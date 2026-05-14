use std::{
    alloc::{self, Layout},
    mem,
    ptr::NonNull,
};

pub struct RawVec<T> {
    ptr: NonNull<T>,
    pub cap: usize,
}

unsafe impl<T: Send> Send for RawVec<T> {}
unsafe impl<T: Sync> Sync for RawVec<T> {}

impl<T> RawVec<T> {
    pub fn new() -> Self {
        let cap = if mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };

        RawVec {
            ptr: NonNull::dangling(), // doubles as unallocated and zero-sized allocation
            cap,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(mem::size_of::<T>() != 0, "with_capacity for ZST");

        let mut raw_vec = RawVec::new();
        let layout = Layout::array::<T>(capacity).expect("Fatal: Allocation too large");
        raw_vec.allocate(capacity, layout);
        raw_vec
    }

    pub fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub fn grow(&mut self) {
        // since we set the capacity to usize::MAX when T has size 0,
        // getting to here necessarily means the Vec is overfull.
        assert!(mem::size_of::<T>() != 0, "capacity overflow");

        let new_cap = if self.cap == 0 { 1 } else { 2 * self.cap }; // cannot overflow since self.cap <= isize::MAX
        let new_layout = Layout::array::<T>(new_cap).expect("Fatal: Allocation too large");

        self.allocate(new_cap, new_layout);
    }

    fn allocate(&mut self, capacity: usize, layout: Layout) {
        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, layout.size()) }
        };

        // If allocation fails, new_ptr will be null, abort
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => alloc::handle_alloc_error(layout),
        };
        self.cap = capacity;
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 && mem::size_of::<T>() != 0 {
            unsafe {
                alloc::dealloc(
                    self.ptr.as_ptr() as *mut u8,
                    Layout::array::<T>(self.cap).unwrap(),
                );
            }
        }
    }
}

impl<T: PartialEq> PartialEq for RawVec<T> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            *self.ptr() == *other.ptr()
        }
    }
}
