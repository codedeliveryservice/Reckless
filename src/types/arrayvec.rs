use std::{mem::MaybeUninit, ops::Index};

#[derive(Clone)]
pub struct ArrayVec<T: Copy, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T: Copy, const N: usize> ArrayVec<T, N> {
    pub const fn new() -> Self {
        let data: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
        Self { data, len: 0 }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: usize) -> &T {
        debug_assert!(index < self.len);

        unsafe { &*self.data.get_unchecked(index).as_ptr() }
    }

    pub fn push(&mut self, value: T) {
        debug_assert!(self.len < N);

        unsafe { self.data[self.len].as_mut_ptr().write(value) };
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe { Some(std::ptr::read(self.data[self.len].as_ptr())) }
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        unsafe {
            let value = std::ptr::read(self.data[index].as_ptr());

            self.len -= 1;
            std::ptr::copy(self.data[self.len].as_ptr(), self.data[index].as_mut_ptr(), 1);

            value
        }
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast(), self.len) }.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.len) }.iter_mut()
    }
}

impl<const N: usize, T: Copy> Index<usize> for ArrayVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.data[index].as_ptr() }
    }
}
