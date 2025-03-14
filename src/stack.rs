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

#[derive(Copy, Clone)]
pub struct Stack {
    data: *mut StackEntry,
}

impl Stack {
    pub fn new(data: &[StackEntry]) -> Self {
        Self { data: data.as_ptr() as *mut _ }
    }

    pub fn next(&self) -> Stack {
        Stack { data: unsafe { self.data.offset(1) } }
    }
}

impl Index<isize> for Stack {
    type Output = StackEntry;

    fn index(&self, index: isize) -> &Self::Output {
        unsafe { &*self.data.offset(index) }
    }
}

impl IndexMut<isize> for Stack {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        unsafe { &mut *self.data.offset(index) }
    }
}

impl Deref for Stack {
    type Target = StackEntry;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}
