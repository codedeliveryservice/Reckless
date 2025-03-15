use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MAX_MOVES},
};

#[derive(PartialEq)]
pub enum Stage {
    HashMove,
    Others,
}

pub struct MovePicker {
    moves: ArrayVec<Move, MAX_MOVES>,
    scores: [i32; MAX_MOVES],
    tt_move: Move,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        let moves = td.board.generate_all_moves();
        let scores = score_moves(td, &moves, -110);

        Self { moves, scores, tt_move, stage: Stage::HashMove }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        let moves = if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() };
        let scores = score_moves(td, &moves, threshold);

        Self { moves, scores, tt_move: Move::NULL, stage: Stage::Others }
    }

    pub fn next(&mut self) -> Option<(Move, i32)> {
        if self.moves.len() == 0 {
            return None;
        }

        if self.stage == Stage::HashMove {
            self.stage = Stage::Others;

            for i in 0..self.moves.len() {
                if self.moves[i] == self.tt_move {
                    let score = self.scores[i];
                    self.scores.swap(i, self.moves.len() - 1);
                    return Some((self.moves.swap_remove(i), score));
                }
            }
        }

        let mut index = 0;
        for i in 1..self.moves.len() {
            if self.scores[i] > self.scores[index] {
                index = i;
            }
        }

        let score = self.scores[index];
        self.scores.swap(index, self.moves.len() - 1);
        Some((self.moves.swap_remove(index), score))
    }
}

fn score_moves(td: &ThreadData, moves: &ArrayVec<Move, MAX_MOVES>, threshold: i32) -> [i32; MAX_MOVES] {
    let mut scores = [0; MAX_MOVES];

    for (i, &mv) in moves.iter().enumerate() {
        if mv.is_noisy() {
            let captured = td.board.piece_on(mv.to()).piece_type();

            scores[i] = if td.board.see(mv, threshold) { 1 << 20 } else { -(1 << 20) };

            scores[i] += PIECE_VALUES[captured as usize % 6] * 32;

            scores[i] += td.noisy_history.get(&td.board, mv);
        } else {
            scores[i] = td.quiet_history.get(&td.board, mv);

            scores[i] += td.conthist(1, mv);
            scores[i] += td.conthist(2, mv);
        }
    }

    scores
}
