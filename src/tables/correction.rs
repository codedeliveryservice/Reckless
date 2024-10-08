use crate::{board::Board, types::Color};

const SIZE: usize = 32768;
const GRAIN: i32 = 256;

const MAX_DELTA: i32 = 64;
const SMOOTHING_FACTOR: i32 = 1024;

pub struct CorrectionHistory {
    table: Box<[[i32; SIZE]; Color::NUM]>,
}

impl CorrectionHistory {
    pub fn get(&self, board: &Board) -> i32 {
        self.table[board.side_to_move()][index(board)] / GRAIN
    }

    pub fn update(&mut self, board: &mut Board, depth: i32, delta: i32) {
        let entry = &mut self.table[board.side_to_move()][index(board)];

        let delta = delta.clamp(-MAX_DELTA, MAX_DELTA) * GRAIN;
        let weight = (10 * depth).min(128);

        *entry = (*entry * (SMOOTHING_FACTOR - weight) + delta * weight) / SMOOTHING_FACTOR;
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { table: Box::new([[0; SIZE]; Color::NUM]) }
    }
}

fn index(board: &Board) -> usize {
    board.pawn_key() as usize % SIZE
}
