use std::ops::{Index, IndexMut};

use crate::types::{Move, Piece, Score, MAX_PLY};

pub struct Stack {
    data: [StackEntry; MAX_PLY + 8],
}

impl Default for Stack {
    fn default() -> Self {
        Self { data: [StackEntry::default(); MAX_PLY + 8] }
    }
}

#[derive(Copy, Clone)]
pub struct StackEntry {
    pub mv: Move,
    pub piece: Piece,
    pub static_eval: i32,
    pub in_check: bool,
    pub excluded: Move,
    pub tt_pv: bool,
    pub cutoff_count: i32,
    pub move_count: i32,
    pub reduction: i32,
    pub conthist: *mut [[i16; 64]; 13],
    pub contcorrhist: *mut [[i16; 64]; 13],
}

unsafe impl Send for StackEntry {}

impl Default for StackEntry {
    fn default() -> Self {
        Self {
            mv: Move::NULL,
            piece: Piece::None,
            static_eval: Score::NONE,
            in_check: false,
            excluded: Move::NULL,
            tt_pv: false,
            cutoff_count: 0,
            move_count: 0,
            reduction: 0,
            conthist: std::ptr::null_mut(),
            contcorrhist: std::ptr::null_mut(),
        }
    }
}

impl Index<usize> for Stack {
    type Output = StackEntry;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.data.get_unchecked(index) }
    }
}

impl IndexMut<usize> for Stack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { self.data.get_unchecked_mut(index) }
    }
}
