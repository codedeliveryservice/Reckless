use crate::{board::Board, types::Color};

const CORR_SIZE: usize = 32768;
const CORR_GRAIN: i32 = 128;
const CORR_MAX: i32 = 64;
const MAX_CORR_HISTORY: i32 = CORR_GRAIN * CORR_MAX;

pub struct CorrectionHistory {
    table: Box<[[i32; CORR_SIZE]; Color::NUM]>,
}

impl CorrectionHistory {
    pub fn get(&self, board: &Board) -> i32 {
        self.table[board.side_to_move()][index(board)] / CORR_GRAIN
    }

    pub fn update(&mut self, board: &mut Board, depth: i32, delta: i32) {
        let entry = &mut self.table[board.side_to_move()][index(board)];
        let weight = (8 * depth).min(128);

        *entry = (*entry + delta * weight).clamp(-MAX_CORR_HISTORY, MAX_CORR_HISTORY);
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { table: Box::new([[0; CORR_SIZE]; Color::NUM]) }
    }
}

fn index(board: &Board) -> usize {
    board.pawn_key() as usize % CORR_SIZE
}
