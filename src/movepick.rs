use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MAX_MOVES},
};

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    GoodNoisy,
    Quiets,
    BadNoisy,
}

pub struct MovePicker {
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    moves: ArrayVec<Move, MAX_MOVES>,
    scores: [i32; MAX_MOVES],
    threshold: i32,
    tt_move: Move,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        let moves = td.board.generate_all_moves();
        let scores = score_moves(td, &moves);

        Self {
            bad_noisy: ArrayVec::new(),
            moves,
            scores,
            threshold: -110,
            tt_move,
            stage: if tt_move != Move::NULL { Stage::HashMove } else { Stage::GoodNoisy },
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        let moves = if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() };
        let scores = score_moves(td, &moves);

        Self {
            bad_noisy: ArrayVec::new(),
            moves,
            scores,
            threshold,
            tt_move: Move::NULL,
            stage: Stage::GoodNoisy,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<Move> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::GoodNoisy;

            for (index, &mv) in self.moves.as_slice().iter().enumerate() {
                if mv == self.tt_move {
                    self.moves.swap_remove(index);
                    self.scores.swap(index, self.moves.len());

                    return Some(mv);
                }
            }
        }

        if self.stage == Stage::GoodNoisy {
            loop {
                let index =
                    match (0..self.moves.len()).filter(|&i| self.moves[i].is_noisy()).max_by_key(|&i| self.scores[i]) {
                        Some(index) => index,
                        None => {
                            self.stage = Stage::Quiets;
                            break;
                        }
                    };

                self.scores.swap(index, self.moves.len() - 1);
                let mv = self.moves.swap_remove(index);

                if !td.board.see(mv, self.threshold) {
                    self.bad_noisy.push(mv);
                    continue;
                }

                return Some(mv);
            }
        }

        if self.stage == Stage::Quiets {
            if let Some(index) = (0..self.moves.len()).max_by_key(|&i| self.scores[i]) {
                self.scores.swap(index, self.moves.len() - 1);
                return Some(self.moves.swap_remove(index));
            }

            self.stage = Stage::BadNoisy;
        }

        // Stage::BadNoisy
        if let Some(index) = (0..self.bad_noisy.len()).max_by_key(|&i| self.scores[i]) {
            let mv = self.bad_noisy[index];

            self.bad_noisy.swap_remove(index);
            self.scores.swap(index, self.bad_noisy.len());

            return Some(mv);
        }

        None
    }
}

fn score_moves(td: &ThreadData, moves: &ArrayVec<Move, MAX_MOVES>) -> [i32; MAX_MOVES] {
    let mut scores = [0; MAX_MOVES];

    for (i, &mv) in moves.iter().enumerate() {
        if mv.is_noisy() {
            let captured = td.board.piece_on(mv.to()).piece_type();

            scores[i] = 1 << 20;

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
