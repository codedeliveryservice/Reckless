use crate::{
    board::Board,
    types::{ArrayVec, Color, Move},
};

fn bonus(depth: i32) -> i32 {
    (128 * depth - 64).min(1280)
}

pub struct QuietHistory {
    // [side_to_move][from_threated][to_threated][from][to]
    entries: Box<[[[[[i32; 64]; 64]; 2]; 2]; 2]>,
}

impl QuietHistory {
    const MAX_HISTORY: i32 = 8192;

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        let from_threated = board.is_threatened(mv.from()) as usize;
        let to_threated = board.is_threatened(mv.to()) as usize;

        self.entries[board.side_to_move()][from_threated][to_threated][mv.from()][mv.to()]
    }

    pub fn update(&mut self, board: &Board, best_move: Move, quiet_moves: &ArrayVec<Move, 32>, depth: i32) {
        let bonus = bonus(depth);

        self.update_single(board, best_move, bonus);

        for &mv in quiet_moves.iter() {
            self.update_single(board, mv, -bonus);
        }
    }

    fn update_single(&mut self, board: &Board, mv: Move, bonus: i32) {
        let from_threated = board.is_threatened(mv.from()) as usize;
        let to_threated = board.is_threatened(mv.to()) as usize;

        let entry = &mut self.entries[board.side_to_move()][from_threated][to_threated][mv.from()][mv.to()];
        *entry += bonus - bonus.abs() * (*entry) / Self::MAX_HISTORY;
    }
}

impl Default for QuietHistory {
    fn default() -> Self {
        Self { entries: Box::new([[[[[0; 64]; 64]; 2]; 2]; 2]) }
    }
}

pub struct NoisyHistory {
    // [piece][to][captured_piece_type]
    entries: Box<[[[i32; 7]; 64]; 12]>,
}

impl NoisyHistory {
    const MAX_HISTORY: i32 = 12288;

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        self.entries[board.piece_on(mv.from())][mv.to()][board.piece_on(mv.to()).piece_type()]
    }

    pub fn update(&mut self, board: &Board, best_move: Move, noisy_moves: &ArrayVec<Move, 32>, depth: i32) {
        let bonus = bonus(depth);

        self.update_single(board, best_move, bonus);

        for &mv in noisy_moves.iter() {
            self.update_single(board, mv, -bonus);
        }
    }

    fn update_single(&mut self, board: &Board, mv: Move, bonus: i32) {
        let entry = &mut self.entries[board.piece_on(mv.from())][mv.to()][board.piece_on(mv.to()).piece_type()];
        *entry += bonus - bonus.abs() * (*entry) / Self::MAX_HISTORY;
    }
}

impl Default for NoisyHistory {
    fn default() -> Self {
        Self { entries: Box::new([[[0; 7]; 64]; 12]) }
    }
}

pub struct CorrectionHistory {
    // [side_to_move][key]
    entries: Box<[[i32; Self::SIZE]; 2]>,
}

impl CorrectionHistory {
    const GRAIN: i32 = 256;
    const LIMIT: i32 = Self::GRAIN * 64;

    const SIZE: usize = 65536;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        self.entries[stm][key as usize & Self::MASK] / Self::GRAIN
    }

    pub fn update(&mut self, stm: Color, key: u64, depth: i32, diff: i32) {
        let weight = (8 * depth + 8).min(96);

        let entry = &mut self.entries[stm][key as usize & Self::MASK];
        let weighted_average = (*entry * (1024 - weight) + diff * weight * Self::GRAIN) / 1024;

        *entry = (weighted_average).clamp(-Self::LIMIT, Self::LIMIT);
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { entries: Box::new([[0; Self::SIZE]; 2]) }
    }
}
