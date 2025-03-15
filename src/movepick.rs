use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveEntry, MoveList},
};

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    Scoring,
    GoodNoisy,
    Quiets,
    BadNoisy,
}

pub struct MovePicker {
    moves: MoveList,
    bad_noises: MoveList,
    tt_move: Move,
    threshold: i32,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        Self {
            moves: td.board.generate_all_moves(),
            bad_noises: MoveList::new(),
            tt_move,
            threshold: -110,
            stage: Stage::HashMove,
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        Self {
            moves: if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() },
            bad_noises: MoveList::new(),
            tt_move: Move::NULL,
            threshold,
            stage: Stage::Scoring,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<Move> {
        if self.moves.is_empty() {
            self.stage = Stage::BadNoisy;

            if self.bad_noises.is_empty() {
                return None;
            }
        }

        if self.stage == Stage::HashMove {
            self.stage = Stage::Scoring;

            for i in 0..self.moves.len() {
                if self.moves[i].mv == self.tt_move {
                    return Some(self.moves.remove(i));
                }
            }
        }

        if self.stage == Stage::Scoring {
            self.stage = Stage::GoodNoisy;
            self.score_moves(td);
        }

        if self.stage == Stage::GoodNoisy {
            let index = self.select_next();
            let mv = self.moves[index].mv;

            if mv.is_noisy() {
                self.moves.remove(index);

                if td.board.see(mv, self.threshold) {
                    return Some(mv);
                }

                self.bad_noises.push(mv);
                return self.next(td);
            }

            self.stage = Stage::Quiets;
            return self.next(td);
        }

        if self.stage == Stage::Quiets {
            let index = self.select_next();
            return Some(self.moves.remove(index));
        }

        if self.bad_noises.is_empty() {
            None
        } else {
            Some(self.bad_noises.remove(0))
        }
    }

    fn score_moves(&mut self, td: &ThreadData) {
        for MoveEntry { mv, score } in self.moves.iter_mut() {
            if mv.is_noisy() {
                let captured = td.board.piece_on(mv.to()).piece_type();

                *score = 1 << 20;

                *score += PIECE_VALUES[captured as usize % 6] * 32;
                *score += td.noisy_history.get(&td.board, *mv);
            } else {
                *score = td.quiet_history.get(&td.board, *mv);

                *score += td.conthist(1, *mv);
                *score += td.conthist(2, *mv);
            }
        }
    }

    fn select_next(&mut self) -> usize {
        let mut index = 0;
        for i in 1..self.moves.len() {
            if self.moves[i].score > self.moves[index].score {
                index = i;
            }
        }
        index
    }
}
