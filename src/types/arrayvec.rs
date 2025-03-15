use std::{mem::MaybeUninit, ops::Index};

pub struct ArrayVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> ArrayVec<T, N> {
    pub const fn new() -> Self {
        let data: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        Self { data, len: 0 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }

    pub fn push(&mut self, value: T) {
        debug_assert!(self.len < N);

        unsafe { self.data[self.len].as_mut_ptr().write(value) };
        self.len += 1;
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        unsafe {
            let value = std::ptr::read(self.data[index].as_ptr());

            self.len -= 1;
            std::ptr::copy(self.data[self.len].as_ptr(), self.data[index].as_mut_ptr(), 1);

            value
        }
    }
}

impl<const N: usize, T> Index<usize> for ArrayVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.data[index].as_ptr() }
    }
}
