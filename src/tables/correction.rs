use crate::{board::Board, types::Color};

const SIZE: usize = 32768;
const GRAIN: i32 = 256;
const SCALE: i32 = 256;
const MAX: i32 = GRAIN * 32;

#[derive(Clone)]
pub struct CorrectionHistory {
    table: Box<[[i32; SIZE]; Color::NUM]>,
}

impl CorrectionHistory {
    pub fn get(&self, board: &Board) -> i32 {
        self.table[board.side_to_move()][index(board)] / GRAIN
    }

    pub fn update(&mut self, board: &mut Board, depth: i32, delta: i32) {
        let entry = &mut self.table[board.side_to_move()][index(board)];
        let delta = delta * GRAIN;

        let weight = (depth + 1).min(16);
        let change = *entry * (SCALE - weight) + delta * weight;

        *entry = (change / SCALE).clamp(-MAX, MAX);
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
