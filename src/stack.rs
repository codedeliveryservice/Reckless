use std::ops::{Index, IndexMut};

use crate::types::{Move, Piece, Score, MAX_PLY};

pub struct Stack {
    data: [StackEntry; MAX_PLY + 16],
}

impl Stack {
    pub fn sentinel(&mut self) -> &mut StackEntry {
        unsafe { self.data.get_unchecked_mut(0) }
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self { data: [StackEntry::default(); MAX_PLY + 16] }
    }
}

#[derive(Copy, Clone)]
pub struct StackEntry {
    pub mv: Move,
    pub piece: Piece,
    pub static_eval: i32,
    pub excluded: Move,
    pub tt_move: Move,
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
        static mut SENTINEL1: [[i16; 64]; 13] = [[0; 64]; 13];
        static mut SENTINEL2: [[i16; 64]; 13] = [[0; 64]; 13];

        Self {
            mv: Move::NULL,
            piece: Piece::None,
            static_eval: Score::NONE,
            excluded: Move::NULL,
            tt_move: Move::NULL,
            tt_pv: false,
            cutoff_count: 0,
            move_count: 0,
            reduction: 0,
            conthist: &raw mut SENTINEL1,
            contcorrhist: &raw mut SENTINEL2,
        }
    }
}

impl Index<isize> for Stack {
    type Output = StackEntry;

    fn index(&self, index: isize) -> &Self::Output {
        debug_assert!(index + 8 >= 0 && index < MAX_PLY as isize + 16);
        unsafe { self.data.get_unchecked((index + 8) as usize) }
    }
}

impl IndexMut<isize> for Stack {
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        debug_assert!(index + 8 >= 0 && index < MAX_PLY as isize + 16);
        unsafe { self.data.get_unchecked_mut((index + 8) as usize) }
    }
}
