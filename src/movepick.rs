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
}

#[derive(Eq, PartialEq)]
pub enum MovePickerKind {
    Normal,
    Probcut,
    QSearch,
}

pub trait MovePickerStrategy {
    const KIND: MovePickerKind;
}

pub struct NormalPicker;
impl MovePickerStrategy for NormalPicker {
    const KIND: MovePickerKind = MovePickerKind::Normal;
}

pub struct ProbcutPicker;
impl MovePickerStrategy for ProbcutPicker {
    const KIND: MovePickerKind = MovePickerKind::Probcut;
}

pub struct QSearchPicker;
impl MovePickerStrategy for QSearchPicker {
    const KIND: MovePickerKind = MovePickerKind::QSearch;
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    threshold: i32,
    stage: Stage,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_idx: usize,
}

impl MovePicker {
    pub fn new<T: MovePickerStrategy>(tt_move: Move, threshold: i32) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            threshold,
            stage: if T::KIND == MovePickerKind::Normal && tt_move.is_some() {
                Stage::HashMove
            } else {
                Stage::GenerateNoisy
            },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub fn next<T: MovePickerStrategy>(&mut self, td: &ThreadData, skip_quiets: bool) -> Option<Move> {
        if T::KIND == MovePickerKind::Normal && self.stage == Stage::HashMove {
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

                let threshold = match T::KIND {
                    MovePickerKind::Probcut => self.threshold,
                    _ => -entry.score / 34 + 107,
                };

                if !td.board.see(entry.mv, threshold) {
                    if T::KIND != MovePickerKind::Probcut {
                        self.bad_noisy.push(entry.mv);
                    }

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
        let threats = td.board.threats();

        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv == self.tt_move {
                entry.score = -32768;
                continue;
            }

            let captured =
                if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

            entry.score = 2238 * PIECE_VALUES[captured] / 128
                + 909 * td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured) / 1024;
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
}
