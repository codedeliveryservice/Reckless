use crate::{
    board::Board,
    types::{ArrayVec, Move},
};

fn bonus(depth: i32) -> i32 {
    (128 * depth - 64).min(1280)
}

pub struct MainHistory {
    // [side_to_move][from_threated][to_threated][from_to]
    entries: Box<[[[[i32; 64 * 64]; 2]; 2]; 2]>,
}

impl MainHistory {
    const MAX_HISTORY: i32 = 8192;

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        let from_threated = board.is_threatened(mv.from()) as usize;
        let to_threated = board.is_threatened(mv.to()) as usize;

        self.entries[board.side_to_move()][from_threated][to_threated][mv.from_to()]
    }

    pub fn update(&mut self, board: &Board, best_move: Move, quiet_moves: ArrayVec<Move, 32>, depth: i32) {
        let bonus = bonus(depth);

        self.update_single(board, best_move, bonus);

        for &mv in quiet_moves.iter() {
            self.update_single(board, mv, -bonus);
        }
    }

    fn update_single(&mut self, board: &Board, mv: Move, bonus: i32) {
        let from_threated = board.is_threatened(mv.from()) as usize;
        let to_threated = board.is_threatened(mv.to()) as usize;

        let entry = &mut self.entries[board.side_to_move()][from_threated][to_threated][mv.from_to()];
        *entry += bonus - bonus.abs() * (*entry) / Self::MAX_HISTORY;
    }
}

impl Default for MainHistory {
    fn default() -> Self {
        MainHistory { entries: Box::new([[[[0; 64 * 64]; 2]; 2]; 2]) }
    }
}
