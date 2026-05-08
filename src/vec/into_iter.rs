use super::raw_val_iter::RawValIter;
use super::raw_vec::RawVec;

pub struct IntoIter<T> {
    _buf: RawVec<T>, // unused, just needs to live
    iter: RawValIter<T>,
}

impl<T> IntoIter<T> {
    pub fn new(iter: RawValIter<T>, buf: RawVec<T>) -> Self {
        IntoIter { _buf: buf, iter }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for _ in &mut *self {}
    }
}
