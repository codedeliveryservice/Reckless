use std::ops::{Index, IndexMut};

use crate::types::{MAX_PLY, Move, Piece, Score};

#[repr(C, align(64))]
pub struct Stack {
    data: [StackEntry; MAX_PLY + 16],
    sentinel: [[i16; 64]; 13],
}

impl Stack {
    pub fn sentinel(&mut self) -> &mut StackEntry {
        unsafe { self.data.get_unchecked_mut(0) }
    }
}

impl Default for Stack {
    fn default() -> Self {
        let mut stack = Self {
            data: [StackEntry::default(); MAX_PLY + 16],
            sentinel: [[0; 64]; 13],
        };

        let ptr = &raw mut stack.sentinel;
        for entry in &mut stack.data {
            entry.conthist = ptr;
            entry.contcorrhist = ptr;
        }
        stack
    }
}

#[derive(Copy, Clone)]
pub struct StackEntry {
    pub mv: Move,
    pub eval: i32,
    pub reduction: i16,
    pub cutoff_count: i16,
    pub excluded: Move,
    pub tt_move: Move,
    pub move_count: u8,
    piece_and_flags: u8,
    pub conthist: *mut [[i16; 64]; 13],
    pub contcorrhist: *mut [[i16; 64]; 13],
}

unsafe impl Send for StackEntry {}

impl StackEntry {
    const PIECE_MASK: u8 = 0b0000_1111;
    const PV_FLAG: u8 = 0b0001_0000;

    pub fn set_piece(&mut self, piece: Piece) {
        self.piece_and_flags = (self.piece_and_flags & !Self::PIECE_MASK) | (piece as u8);
    }

    pub fn set_tt_pv(&mut self, is_pv: bool) {
        if is_pv {
            self.piece_and_flags |= Self::PV_FLAG;
        } else {
            self.piece_and_flags &= !Self::PV_FLAG;
        }
    }

    pub fn piece(&self) -> Piece {
        unsafe { std::mem::transmute(self.piece_and_flags & Self::PIECE_MASK) }
    }

    pub fn is_tt_pv(&self) -> bool {
        (self.piece_and_flags & Self::PV_FLAG) != 0
    }
}

impl Default for StackEntry {
    fn default() -> Self {
        Self {
            mv: Move::NULL,
            eval: Score::NONE,
            excluded: Move::NULL,
            tt_move: Move::NULL,
            cutoff_count: 0,
            move_count: 0,
            reduction: 0,
            piece_and_flags: 0,
            conthist: std::ptr::null_mut(),
            contcorrhist: std::ptr::null_mut(),
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
