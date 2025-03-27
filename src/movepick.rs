use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MoveList, MAX_MOVES},
};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    GenerateNoisy,
    GoodNoisy,
    GenerateQuiet,
    Quiet,
    BadNoisy,
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    killer: Move,
    threshold: i32,
    stage: Stage,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_idx: usize,
    in_check: bool,
}

impl MovePicker {
    pub const fn new(killer: Move, tt_move: Move, in_check: bool) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            killer,
            threshold: -110,
            stage: if tt_move.is_valid() { Stage::HashMove } else { Stage::GenerateNoisy },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
            in_check,
        }
    }

    pub const fn new_noisy(threshold: i32, in_check: bool) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            killer: Move::NULL,
            threshold,
            stage: Stage::GenerateNoisy,
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
            in_check,
        }
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn next(&mut self, td: &ThreadData, skip_quiets: bool) -> Option<Move> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::GenerateNoisy;

            if td.board.is_pseudo_legal(self.tt_move) {
                return Some(self.tt_move);
            }
        }

        if self.stage == Stage::GenerateNoisy {
            self.stage = Stage::GoodNoisy;
            td.board.append_noisy_moves(&mut self.list);
            self.score_noisy(td);
        }

        if self.stage == Stage::GoodNoisy {
            while !self.list.is_empty() {
                let mut index = 0;
                for i in 1..self.list.len() {
                    if self.list[i].score > self.list[index].score {
                        index = i;
                    }
                }

                let entry = self.list.remove(index);
                if entry.mv == self.tt_move {
                    continue;
                }

                if !td.board.see(entry.mv, self.threshold) {
                    self.bad_noisy.push(entry.mv);
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = Stage::GenerateQuiet;
        }

        if self.stage == Stage::GenerateQuiet {
            if !skip_quiets {
                self.stage = Stage::Quiet;
                td.board.append_quiet_moves(&mut self.list);
                self.score_quiet(td);
            } else {
                self.stage = Stage::BadNoisy;
            }
        }

        if self.stage == Stage::Quiet {
            if !skip_quiets {
                while !self.list.is_empty() {
                    let mut index = 0;
                    for i in 1..self.list.len() {
                        if self.list[i].score > self.list[index].score {
                            index = i;
                        }
                    }

                    let entry = self.list.remove(index);
                    if entry.mv == self.tt_move {
                        continue;
                    }

                    return Some(entry.mv);
                }
            }

            self.stage = Stage::BadNoisy;
        }

        // Stage::BadNoisy
        while self.bad_noisy_idx < self.bad_noisy.len() {
            let mv = self.bad_noisy[self.bad_noisy_idx];
            self.bad_noisy_idx += 1;

            if mv == self.tt_move {
                continue;
            }

            return Some(mv);
        }

        None
    }

    fn score_noisy(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let captured = td.board.piece_on(entry.mv.to()).piece_type();

            entry.score = PIECE_VALUES[captured as usize % 6] * 32;
            entry.score += td.noisy_history.get(&td.board, entry.mv);
        }
    }

    fn score_quiet(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            entry.score = (1 << 18) * (mv == self.killer) as i32
                + td.quiet_history.get(&td.board, mv)
                + td.conthist(1, mv)
                + td.conthist(2, mv) * !self.in_check as i32
                + td.conthist(4, mv) / 2 * !self.in_check as i32;
        }
    }
}
