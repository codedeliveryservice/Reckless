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
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len).iter() }
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len).iter_mut() }
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

impl<T, const N: usize> Index<usize> for ArrayVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(index < self.len);
        unsafe { &*self.data.get_unchecked(index).as_ptr() }
    }
}
