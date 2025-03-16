use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MoveList, MAX_MOVES},
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
    list: MoveList,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_index: usize,
    tt_move: Move,
    threshold: i32,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        Self {
            list: td.board.generate_all_moves(),
            bad_noisy: ArrayVec::new(),
            bad_noisy_index: 0,
            tt_move,
            threshold: -110,
            stage: Stage::HashMove,
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        Self {
            list: if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() },
            bad_noisy: ArrayVec::new(),
            bad_noisy_index: 0,
            tt_move: Move::NULL,
            threshold,
            stage: Stage::Scoring,
        }
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<(Move, i32)> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::Scoring;

            let index = self.list.iter().position(|entry| entry.mv == self.tt_move);
            if let Some(index) = index {
                let mv = self.list.remove(index);
                return Some((mv, 1 << 21));
            }
        }

        if self.stage == Stage::Scoring {
            self.stage = Stage::GoodNoisy;
            self.score_moves(td);
        }

        if self.stage == Stage::GoodNoisy {
            loop {
                let index = match self.find_best() {
                    Some(index) => index,
                    None => {
                        self.stage = Stage::Quiets;
                        break;
                    }
                };

                let mv = self.list[index].mv;

                if !mv.is_noisy() {
                    self.stage = Stage::Quiets;
                    break;
                }

                self.list.remove(index);

                if !td.board.see(mv, self.threshold) {
                    self.bad_noisy.push(mv);
                    continue;
                }

                return Some((mv, 1 << 20));
            }
        }

        if self.stage == Stage::Quiets {
            let index = match self.find_best() {
                Some(index) => index,
                None => {
                    self.stage = Stage::BadNoisy;
                    return self.next(td);
                }
            };

            let mv = self.list[index].mv;
            let score = self.list[index].score;

            self.list.remove(index);
            return Some((mv, score));
        }

        // Stage::BadNoisy
        if self.bad_noisy_index < self.bad_noisy.len() {
            let mv = self.bad_noisy[self.bad_noisy_index];
            self.bad_noisy_index += 1;

            return Some((mv, -(1 << 20)));
        }
        None
    }

    fn find_best(&self) -> Option<usize> {
        if self.list.len() == 0 {
            return None;
        }

        let mut index = 0;
        for i in 1..self.list.len() {
            if self.list[i].score > self.list[index].score {
                index = i;
            }
        }
        Some(index)
    }

    fn score_moves(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv.is_noisy() {
                let captured = td.board.piece_on(mv.to()).piece_type();
                entry.score = (1 << 20) + PIECE_VALUES[captured as usize % 6] * 32 + td.noisy_history.get(&td.board, mv);
            } else {
                entry.score = td.quiet_history.get(&td.board, mv) + td.conthist(1, mv) + td.conthist(2, mv);
            }
        }
    }
}
