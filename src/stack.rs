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
    pub excluded: Move,
    pub killer: Move,
    pub tt_pv: bool,
    pub cutoff_count: i32,
    pub reduction: i32,
    pub first_history_ordered_quiet_move: bool,
}

impl Default for StackEntry {
    fn default() -> Self {
        Self {
            mv: Move::NULL,
            piece: Piece::None,
            static_eval: Score::NONE,
            excluded: Move::NULL,
            killer: Move::NULL,
            tt_pv: false,
            cutoff_count: 0,
            reduction: 0,
            first_history_ordered_quiet_move: false,
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
