use crate::{
    board::Board,
    types::{ArrayVec, Move},
};

pub struct MainHistory {
    // [side_to_move][from_to]
    entries: Box<[[i32; 64 * 64]; 2]>,
}

impl MainHistory {
    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        self.entries[board.side_to_move()][mv.from_to()]
    }

    pub fn update(&mut self, board: &Board, best_move: Move, quiet_moves: ArrayVec<Move, 32>, depth: i32) {
        self.entries[board.side_to_move()][best_move.from_to()] += depth;

        for mv in quiet_moves.iter() {
            self.entries[board.side_to_move()][mv.from_to()] -= depth;
        }
    }
}

impl Default for MainHistory {
    fn default() -> Self {
        MainHistory { entries: Box::new([[0; 64 * 64]; 2]) }
    }
}
