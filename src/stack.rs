use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::types::{Move, Piece, Score};

#[derive(Copy, Clone)]
pub struct StackEntry {
    pub ply: usize,
    pub mv: Move,
    pub piece: Piece,
    pub eval: i32,
    pub excluded: Move,
    pub tt_pv: bool,
    pub cutoff_count: i32,
}

impl Default for StackEntry {
    fn default() -> Self {
        Self {
            ply: 0,
            mv: Move::NULL,
            piece: Piece::None,
            eval: Score::NONE,
            excluded: Move::NULL,
            tt_pv: false,
            cutoff_count: 0,
        }
    }
}

pub struct Stack<'a> {
    data: &'a mut [StackEntry],
}

impl Stack<'_> {
    pub fn new(data: &[StackEntry]) -> Self {
        Self {
            data: unsafe { std::slice::from_raw_parts_mut(data.as_ptr() as *mut _, 4) },
        }
    }

    pub fn clone(&self) -> Stack<'_> {
        Stack::new(&self.data[..])
    }

    pub fn next(&self) -> Stack<'_> {
        Stack::new(&self.data[1..])
    }
}

impl Index<isize> for Stack<'_> {
    type Output = StackEntry;

    fn index(&self, index: isize) -> &Self::Output {
        unsafe { &*self.data.as_ptr().offset(index) }
    }
}

impl IndexMut<isize> for Stack<'_> {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        unsafe { &mut *self.data.as_mut_ptr().offset(index) }
    }
}

impl Deref for Stack<'_> {
    type Target = StackEntry;

    fn deref(&self) -> &Self::Target {
        &self.data[0]
    }
}

impl DerefMut for Stack<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[0]
    }
}
