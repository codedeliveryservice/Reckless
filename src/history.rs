use crate::{
    board::Board,
    types::{Bitboard, Color, Move, Piece, Square},
};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 12];

pub struct QuietHistory {
    // [side_to_move][from_threated][to_threated][from][to]
    entries: Box<[[[FromToHistory<i16>; 2]; 2]; 2]>,
}

impl QuietHistory {
    const MAX_HISTORY: i32 = 8192;

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        let from_threated = board.is_threatened(mv.from()) as usize;
        let to_threated = board.is_threatened(mv.to()) as usize;

        self.entries[board.side_to_move()][from_threated][to_threated][mv.from()][mv.to()] as i32
    }

    pub fn update(&mut self, threats: Bitboard, stm: Color, mv: Move, bonus: i32) {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        let entry = &mut self.entries[stm][from_threated][to_threated][mv.from()][mv.to()];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for QuietHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct NoisyHistory {
    // [piece][to][captured_piece_type]
    entries: Box<PieceToHistory<[i16; 7]>>,
}

impl NoisyHistory {
    const MAX_HISTORY: i32 = 12288;

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        self.entries[board.piece_on(mv.from())][mv.to()][board.piece_on(mv.to()).piece_type()] as i32
    }

    pub fn update(&mut self, board: &Board, mv: Move, bonus: i32) {
        let entry = &mut self.entries[board.piece_on(mv.from())][mv.to()][board.piece_on(mv.to()).piece_type()];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for NoisyHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct CorrectionHistory {
    // [side_to_move][key]
    entries: Box<[[i16; Self::SIZE]; 2]>,
}

impl CorrectionHistory {
    const MAX_HISTORY: i32 = 16384;

    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        (self.entries[stm][key as usize & Self::MASK] / 96) as i32
    }

    pub fn update(&mut self, stm: Color, key: u64, depth: i32, diff: i32) {
        let entry = &mut self.entries[stm][key as usize & Self::MASK];
        let bonus = (diff * depth).clamp(-Self::MAX_HISTORY / 4, Self::MAX_HISTORY / 4);

        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationHistory {
    // [piece][to][continuation_piece][continuation_to]
    entries: Box<[[PieceToHistory<i16>; 64]; 13]>,
}

impl ContinuationHistory {
    const MAX_HISTORY: i32 = 16384;

    pub fn get(&self, piece: Piece, sq: Square, cont_piece: Piece, cont_sq: Square) -> i32 {
        self.entries[piece][sq][cont_piece][cont_sq] as i32
    }

    pub fn update(&mut self, piece: Piece, sq: Square, cont_piece: Piece, cont_sq: Square, bonus: i32) {
        let entry = &mut self.entries[piece][sq][cont_piece][cont_sq];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for ContinuationHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

fn zeroed_box<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::<T>::from_raw(ptr.cast())
    }
}
