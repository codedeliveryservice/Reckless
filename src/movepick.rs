use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MAX_MOVES},
};

#[derive(PartialEq)]
enum Stage {
    HashMove,
    Scoring,
    EverythingElse,
}

pub struct MovePicker {
    moves: ArrayVec<Move, MAX_MOVES>,
    scores: [i32; MAX_MOVES],
    threshold: i32,
    tt_move: Move,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        Self {
            moves: td.board.generate_all_moves(),
            scores: [0; MAX_MOVES],
            threshold: -110,
            tt_move,
            stage: if tt_move != Move::NULL { Stage::HashMove } else { Stage::Scoring },
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        Self {
            moves: if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() },
            scores: [0; MAX_MOVES],
            threshold,
            tt_move: Move::NULL,
            stage: Stage::Scoring,
        }
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<(Move, i32)> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::Scoring;

            for (index, &mv) in self.moves.as_slice().iter().enumerate() {
                if mv == self.tt_move {
                    self.moves.swap_remove(index);
                    self.scores.swap(index, self.moves.len());

                    return Some((mv, 1 << 21));
                }
            }
        }

        if self.stage == Stage::Scoring {
            self.stage = Stage::EverythingElse;

            self.scores = score_moves(&td, &self.moves, self.threshold);
        }

        // Stage::EverythingElse
        if self.moves.len() == 0 {
            return None;
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
