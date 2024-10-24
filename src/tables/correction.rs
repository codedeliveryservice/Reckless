use crate::{board::Board, types::Color};

const SCALE: i32 = 2000;
const GRAIN: i32 = 250;
const LIMIT: i32 = 32;

#[derive(Clone, Default)]
pub struct CorrectionHistory {
    pawn: CorrectionTable,
    minor: CorrectionTable,
}

impl CorrectionHistory {
    pub fn get(&self, board: &Board) -> i32 {
        (self.pawn.get(board, board.pawn_key()) + self.minor.get(board, board.minor_key())) / GRAIN
    }

    pub fn update(&mut self, board: &mut Board, depth: i32, delta: i32) {
        update_entry(self.pawn.get_mut(board, board.pawn_key()), depth, delta);
        update_entry(self.minor.get_mut(board, board.minor_key()), depth, delta);
    }
}

fn update_entry(entry: &mut i32, depth: i32, delta: i32) {
    let weight = weight(depth);
    let value = (*entry * (SCALE - weight) + delta * weight * GRAIN) / SCALE;

    *entry = value.clamp(-LIMIT * GRAIN, LIMIT * GRAIN);
}

fn weight(depth: i32) -> i32 {
    (20 * depth + 10).min(250)
}

#[derive(Clone)]
struct CorrectionTable {
    table: Box<[[i32; Self::SIZE]; Color::NUM]>,
}

impl CorrectionTable {
    // The size has to be a power of two.
    const SIZE: usize = 16384;

    pub fn get(&self, board: &Board, key: u64) -> i32 {
        self.table[board.side_to_move()][key as usize & (Self::SIZE - 1)]
    }

    pub fn get_mut(&mut self, board: &Board, key: u64) -> &mut i32 {
        &mut self.table[board.side_to_move()][key as usize & (Self::SIZE - 1)]
    }
}

impl Default for CorrectionTable {
    fn default() -> Self {
        Self { table: Box::new([[0; Self::SIZE]; Color::NUM]) }
    }
}
