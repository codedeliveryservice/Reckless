use crate::{
    board::Board,
    types::{Color, Move, Piece, Square},
};

pub struct CounterMoves {
    table: [[[Move; Square::NUM]; Piece::NUM]; Color::NUM],
}

impl CounterMoves {
    pub fn get(&self, board: &Board) -> Option<Move> {
        if let Some(previous) = self.previous_move(board) {
            let piece = board.get_piece(previous.target()).unwrap();
            return Some(self.table[!board.side_to_move][piece][previous.target()]);
        }
        None
    }

    pub fn update(&mut self, counter: Move, board: &Board) {
        if let Some(previous) = self.previous_move(board) {
            let piece = board.get_piece(previous.target()).unwrap();
            self.table[!board.side_to_move][piece][previous.target()] = counter;
        }
    }

    fn previous_move(&self, board: &Board) -> Option<Move> {
        board.get_last_move().filter(|&m| m != Move::NULL)
    }
}

impl Default for CounterMoves {
    fn default() -> Self {
        CounterMoves {
            table: [[[Move::NULL; Square::NUM]; Piece::NUM]; Color::NUM],
        }
    }
}
