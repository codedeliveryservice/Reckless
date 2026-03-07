use crate::{
    search::NodeType,
    thread::ThreadData,
    types::{ArrayVec, MAX_MOVES, Move, MoveList, PieceType},
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
    threshold: Option<i32>,
    stage: Stage,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_idx: usize,
}

impl MovePicker {
    pub const fn new(tt_move: Move) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            threshold: None,
            stage: if tt_move.is_some() { Stage::HashMove } else { Stage::GenerateNoisy },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_probcut(threshold: i32) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: Some(threshold),
            stage: Stage::GenerateNoisy,
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_qsearch() -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: None,
            stage: Stage::GenerateNoisy,
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn stage(&self) -> Stage {
        self.stage
    }

    pub fn next<NODE: NodeType>(&mut self, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
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
                let index = self.find_best_score_index();
                let entry = &self.list.remove(index);
                if entry.mv == self.tt_move {
                    continue;
                }

                let threshold = self.threshold.unwrap_or_else(|| -entry.score / 46 + 109);
                if !td.board.see(entry.mv, threshold) {
                    self.bad_noisy.push(entry.mv);
                    continue;
                }

                if NODE::ROOT {
                    self.score_noisy(td);
                }

                return Some(entry.mv);
            }

            self.stage = Stage::GenerateQuiet;
        }

        if self.stage == Stage::GenerateQuiet {
            if skip_quiets {
                self.stage = Stage::BadNoisy;
            } else {
                self.stage = Stage::Quiet;
                td.board.append_quiet_moves(&mut self.list);
                self.score_quiet(td, ply);
            }
        }

        if self.stage == Stage::Quiet {
            if !skip_quiets {
                while !self.list.is_empty() {
                    let index = self.find_best_score_index();
                    let entry = &self.list.remove(index);
                    if entry.mv == self.tt_move {
                        continue;
                    }

                    if NODE::ROOT {
                        self.score_quiet(td, ply);
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

    fn find_best_score_index(&self) -> usize {
        let mut best_index = 0;
        let mut best_score = i32::MIN;

        for (index, entry) in self.list.iter().enumerate() {
            if entry.score >= best_score {
                best_index = index;
                best_score = entry.score;
            }
        }

        best_index
    }

    fn score_noisy(&mut self, td: &ThreadData) {
        let threats = td.board.all_threats();

        if td.board.checkers().is_empty() {
            for entry in self.list.iter_mut() {
                let mv = entry.mv;
                let captured =
                    if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

                entry.score =
                    16 * captured.value() + td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured);
            }
        } else {
            //in check
            for entry in self.list.iter_mut() {
                let mv = entry.mv;
                let pt = td.board.piece_on(mv.from()).piece_type();

                entry.score = 10000 - 1000 * pt as i32;
            }
        }
    }

    fn score_quiet(&mut self, td: &ThreadData, ply: isize) {
        let threats = td.board.all_threats();

        let side = td.board.side_to_move();

        let pawn_threats = td.board.piece_threats(PieceType::Pawn);

        let minor_threats =
            pawn_threats | td.board.piece_threats(PieceType::Knight) | td.board.piece_threats(PieceType::Bishop);

        let rook_threats = minor_threats | td.board.piece_threats(PieceType::Rook);

        let threatened = (td.board.our(PieceType::Queen) & rook_threats)
            | (td.board.our(PieceType::Rook) & minor_threats)
            | (td.board.our(PieceType::Knight) & pawn_threats)
            | (td.board.our(PieceType::Bishop) & pawn_threats);

        for entry in self.list.iter_mut() {
            let mv = entry.mv;
            let pt = td.board.piece_on(mv.from()).piece_type();

            if mv == self.tt_move {
                entry.score = i32::MIN;
                continue;
            }

            entry.score = td.quiet_history.get(threats, side, mv)
                + td.conthist(ply, 1, mv)
                + td.conthist(ply, 2, mv)
                + td.conthist(ply, 4, mv)
                + td.conthist(ply, 6, mv);

            // bonus for escaping capture
            if threatened.contains(mv.from()) {
                if pt == PieceType::Queen {
                    entry.score += 20000;
                } else if pt == PieceType::Rook {
                    entry.score += 14000;
                } else if pt != PieceType::Pawn {
                    entry.score += 8000;
                }
            }

            // Bonus for checking moves
            if td.board.checking_squares(td.board.moved_piece(mv).piece_type()).contains(mv.to()) {
                entry.score += 10000;
            }
            // Malus for moving into danger
            else if pt == PieceType::Queen && minor_threats.contains(mv.to()) {
                entry.score -= 10000;
            }
        }
    }
}
