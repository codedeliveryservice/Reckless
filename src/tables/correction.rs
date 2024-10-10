use crate::{board::Board, types::Color};

const SIZE: usize = 32768;
const GRAIN: i32 = 256;
const SCALE: i32 = 2048;
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

        let weight = (3 * depth * depth + 6 * depth + 3).min(350);
        let value = (*entry * (SCALE - weight) + delta * weight * GRAIN) / SCALE;

        *entry = value.clamp(-MAX, MAX);
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
