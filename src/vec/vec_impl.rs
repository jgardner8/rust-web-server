use std::fmt::{Debug, Write};
use std::ops::{Deref, DerefMut};
use std::{mem, ptr};

use super::drain::Drain;
use super::into_iter::IntoIter;
use super::raw_val_iter::RawValIter;
use super::raw_vec::RawVec;

#[derive(PartialEq)]
pub struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        Vec {
            buf: RawVec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Vec {
            buf: RawVec::with_capacity(capacity),
            len: 0,
        }
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.buf.cap {
            self.buf.grow();
        }

        unsafe {
            ptr::write(self.buf.ptr().add(self.len), elem);
        }

        self.len += 1; // out of memory will occur before overflow
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.buf.ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "index out of bounds");
        if self.len == self.buf.cap {
            self.buf.grow();
        }

        let count = self.len - index;
        unsafe {
            let src = self.buf.ptr().add(index);
            let dest = self.buf.ptr().add(index + 1);
            ptr::copy(src, dest, count);

            ptr::write(src, elem);
        }

        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "index out of bounds");

        self.len -= 1;
        let count = self.len - index;

        unsafe {
            let result = ptr::read(self.buf.ptr().add(index));

            let src = self.buf.ptr().add(index + 1);
            let dest = self.buf.ptr().add(index);
            ptr::copy(src, dest, count);

            result
        }
    }

    pub fn drain(&mut self) -> Drain<'_, T> {
        let iter = unsafe { RawValIter::new(self) };

        // this is a mem::forget safety thing. If Drain is forgotten, we just
        // leak the whole Vec's contents. Also we need to do this *eventually*
        // anyway, so why not do it now?
        self.len = 0;

        Drain::new(iter)
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
        // deallocation is handled by RawVec
    }
}

impl<T: Clone> Clone for Vec<T> {
    fn clone(&self) -> Self {
        let mut vec = Vec::with_capacity(self.len);
        for value in self.iter() {
            vec.push(value.clone());
        }
        vec
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.buf.ptr(), self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.buf.ptr(), self.len) }
    }
}

impl<T> IntoIterator for Vec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let iter = RawValIter::new(&self);
            let buf = ptr::read(&self.buf);
            mem::forget(self);

            IntoIter::new(iter, buf)
        }
    }
}

impl<T> FromIterator<T> for Vec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vec = Vec::new();
        for value in iter {
            vec.push(value);
        }
        vec
    }
}

impl<T, const N: usize> From<[T; N]> for Vec<T> {
    fn from(array: [T; N]) -> Self {
        Vec::from_iter(array)
    }
}

impl<T: Debug> Debug for Vec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;

        let mut iter = self.iter().peekable();
        while let Some(value) = iter.next() {
            write!(f, "{:?}", value)?;
            if iter.peek().is_some() {
                f.write_str(", ")?;
            }
        }

        f.write_char(']')?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug() {
        let vec = Vec::from(["1", "2", "3"]);
        let output = format!("{:?}", vec);
        assert_eq!(output, r#"["1", "2", "3"]"#);
    }
}
