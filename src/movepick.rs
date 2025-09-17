use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{ArrayVec, Move, MoveList, PieceType, MAX_MOVES},
};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    GenerateNoisy,
    GoodNoisy,
    GenerateQuiet,
    Quiet,
    BadNoisy,
    EvasionHashMove,
    GenerateEvasions,
    Evasions,
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    threshold: Option<i32>,
    stage: Stage,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_idx: usize,
}

impl MovePicker {
    pub const fn new(in_check: bool, tt_move: Move) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            threshold: None,
            stage: match (in_check, tt_move.is_some()) {
                (true, true) => Stage::EvasionHashMove,
                (true, false) => Stage::GenerateEvasions,
                (false, true) => Stage::HashMove,
                (false, false) => Stage::GenerateNoisy,
            },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_probcut(in_check: bool, threshold: i32) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: Some(threshold),
            stage: match in_check {
                true => Stage::GenerateEvasions,
                false => Stage::GenerateNoisy,
            },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_qsearch(in_check: bool) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: None,
            stage: match in_check {
                true => Stage::GenerateEvasions,
                false => Stage::GenerateNoisy,
            },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn stage(&self) -> Stage {
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

                let entry = &self.list.remove(index);
                if entry.mv == self.tt_move {
                    continue;
                }

                let threshold = self.threshold.unwrap_or_else(|| -entry.score / 36 + 116);
                if !td.board.see(entry.mv, threshold) {
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

                    let entry = &self.list.remove(index);
                    if entry.mv == self.tt_move {
                        continue;
                    }

                    return Some(entry.mv);
                }
            }

            self.stage = Stage::BadNoisy;
        }

        if self.stage == Stage::BadNoisy {
            while self.bad_noisy_idx < self.bad_noisy.len() {
                let mv = self.bad_noisy[self.bad_noisy_idx];
                self.bad_noisy_idx += 1;

                if mv == self.tt_move {
                    continue;
                }

                return Some(mv);
            }
        }

        if self.stage == Stage::EvasionHashMove {
            self.stage = Stage::GenerateEvasions;

            if td.board.is_pseudo_legal(self.tt_move) {
                return Some(self.tt_move);
            }
        }

        if self.stage == Stage::GenerateEvasions {
            self.stage = Stage::Evasions;
            td.board.append_evasion_moves(&mut self.list);
            self.score_evasions(td);
        }

        if self.stage == Stage::Evasions {
            while !self.list.is_empty() {
                let mut index = 0;
                for i in 1..self.list.len() {
                    if !(skip_quiets && self.list[i].mv.is_quiet()) && self.list[i].score > self.list[index].score {
                        index = i;
                    }
                }

                let entry = &self.list.remove(index);
                if entry.mv.is_quiet() && skip_quiets {
                    return None;
                }

                if entry.mv == self.tt_move {
                    continue;
                }

                return Some(entry.mv);
            }

            return None;
        }

        None
    }

    fn score_noisy(&mut self, td: &ThreadData) {
        let threats = td.board.threats();

        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv == self.tt_move {
                entry.score = -32768;
                continue;
            }

            let captured =
                if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

            entry.score = 2009 * PIECE_VALUES[captured] / 128
                + 1067 * td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured) / 1024;
        }
    }

    fn score_quiet(&mut self, td: &ThreadData) {
        let threats = td.board.threats();
        let side = td.board.side_to_move();

        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv == self.tt_move {
                entry.score = -32768;
                continue;
            }

            entry.score = td.quiet_history.get(threats, side, mv)
                + td.conthist(1, mv)
                + td.conthist(2, mv)
                + td.conthist(4, mv)
                + td.conthist(6, mv);
        }
    }

    fn score_evasions(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv.is_noisy() {
                let captured =
                    if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

                entry.score = 1 << 28;
                entry.score += PIECE_VALUES[captured];
            } else {
                entry.score +=
                    td.quiet_history.get(td.board.threats(), td.board.side_to_move(), mv) + td.conthist(1, mv);
            }
        }
    }
}
