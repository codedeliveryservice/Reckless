use crate::{
    thread::ThreadData,
    types::{ArrayVec, Move, MAX_MOVES},
};

pub struct MovePicker {
    moves: ArrayVec<Move, MAX_MOVES>,
    scores: [i32; MAX_MOVES],
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        let moves = td.board.generate_all_moves();
        let scores = score_moves(td, &moves, tt_move);

        Self { moves, scores }
    }

    pub fn new_noisy(td: &ThreadData) -> Self {
        let moves = td.board.generate_capture_moves();
        let scores = score_moves(td, &moves, Move::NULL);

        Self { moves, scores }
    }

    pub fn next(&mut self) -> Option<Move> {
        if self.moves.len() == 0 {
            return None;
        }

        let mut index = 0;
        for i in 1..self.moves.len() {
            if self.scores[i] > self.scores[index] {
                index = i;
            }
        }

        self.scores.swap(index, self.moves.len() - 1);
        Some(self.moves.swap_remove(index))
    }
}

fn score_moves(td: &ThreadData, moves: &ArrayVec<Move, MAX_MOVES>, tt_move: Move) -> [i32; MAX_MOVES] {
    let mut scores = [0; MAX_MOVES];

    for (i, &mv) in moves.iter().enumerate() {
        if mv == tt_move {
            scores[i] = 1 << 21;
            continue;
        }

        if mv.is_noisy() {
            let moving = td.board.piece_on(mv.from()).piece_type();
            let captured = td.board.piece_on(mv.to()).piece_type();

            scores[i] = 1 << 20;

            scores[i] += captured as i32 * 16384;
            scores[i] -= moving as i32;
        } else {
            scores[i] = td.quiet_history.get(&td.board, mv);
        }
    }

    scores
}
