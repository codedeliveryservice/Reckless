use std::{mem::MaybeUninit, ops::Index};

use crate::types::MoveEntry;

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

    pub fn maybe_push(&mut self, mask: bool, value: T) {
        debug_assert!(self.len < N);

        unsafe { self.data[self.len].as_mut_ptr().write(value) };
        self.len += mask as usize;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        unsafe { Some(std::ptr::read(self.data[self.len].as_ptr())) }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        unsafe {
            let value = std::ptr::read(self.data[index].as_ptr());

            self.len -= 1;
            std::ptr::copy(self.data[self.len].as_ptr(), self.data[index].as_mut_ptr(), 1);

            value
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr().cast(), self.len) }.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.len) }.iter_mut()
    }

    #[allow(dead_code)]
    pub unsafe fn unchecked_write<F>(&mut self, op: F)
    where
        F: FnOnce(*mut T) -> usize,
    {
        self.len += op(self.data.get_unchecked_mut(self.len).as_mut_ptr());
    }

    pub unsafe fn as_ptr(&self) -> *const MaybeUninit<T> {
        self.data.as_ptr()
    }
}

impl<const N: usize> ArrayVec<MoveEntry, N> {
    #[cfg(all(target_feature = "avx512vl", target_feature = "avx512vbmi"))]
    pub unsafe fn splat8(&mut self, mask: u32, vector: std::arch::x86_64::__m512i) {
        use std::arch::x86_64::*;

        let count = mask.count_ones() as usize;
        let to_write = _mm512_maskz_compress_epi16(mask, vector);
        let to_write0 = _mm512_cvtepi16_epi64(_mm512_castsi512_si128(to_write));
        _mm512_storeu_si512(self.data.get_unchecked_mut(self.len).as_mut_ptr().cast(), to_write0);
        self.len += count;
    }

    #[cfg(all(target_feature = "avx512vl", target_feature = "avx512vbmi"))]
    pub unsafe fn splat16(&mut self, mask: u32, vector: std::arch::x86_64::__m512i) {
        use std::arch::x86_64::*;

        let count = mask.count_ones() as usize;
        let to_write = _mm512_maskz_compress_epi16(mask, vector);
        let to_write0 = _mm512_cvtepi16_epi64(_mm512_castsi512_si128(to_write));
        let to_write1 = _mm512_cvtepi16_epi64(_mm512_extracti32x4_epi32::<1>(to_write));
        _mm512_storeu_si512(self.data.get_unchecked_mut(self.len).as_mut_ptr().cast(), to_write0);
        _mm512_storeu_si512(self.data.get_unchecked_mut(self.len + 8).as_mut_ptr().cast(), to_write1);
        self.len += count;
    }
}

impl<const N: usize, T: Copy> Index<usize> for ArrayVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.data.get_unchecked(index).as_ptr() }
    }
}
