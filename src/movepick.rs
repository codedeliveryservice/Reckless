use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks_setwise, rook_attacks},
    search::NodeType,
    thread::ThreadData,
    types::{ArrayVec, Bitboard, MAX_MOVES, Move, MoveEntry, MoveList, PieceType},
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
            stage: if tt_move.is_present() { Stage::HashMove } else { Stage::GenerateNoisy },
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

            if td.board.is_legal(self.tt_move) {
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
                let entry = self.get_best_entry();
                if entry.mv == self.tt_move {
                    continue;
                }

                let threshold = self.threshold.unwrap_or_else(|| -entry.score / 45 + 111);
                if !td.board.see(entry.mv, threshold) {
                    self.bad_noisy.push(entry.mv);
                    continue;
                }

                if NODE::ROOT {
                    self.score_noisy(td);
                }

                return Some(entry.mv);
            }

            if skip_quiets {
                self.stage = Stage::BadNoisy;
            } else {
                self.stage = Stage::GenerateQuiet;
            }
        }

        if self.stage == Stage::GenerateQuiet {
            self.stage = Stage::Quiet;
            td.board.append_quiet_moves(&mut self.list);
            self.score_quiet(td, ply);
        }

        if self.stage == Stage::Quiet {
            if !skip_quiets {
                while !self.list.is_empty() {
                    let entry = self.get_best_entry();
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
        if self.bad_noisy_idx < self.bad_noisy.len() {
            let mv = self.bad_noisy[self.bad_noisy_idx];
            self.bad_noisy_idx += 1;
            return Some(mv);
        }

        None
    }

    fn get_best_entry(&mut self) -> MoveEntry {
        let mut best_index = 0;
        let mut best_score = i32::MIN;

        for (index, entry) in self.list.iter().enumerate() {
            if entry.score >= best_score {
                best_index = index;
                best_score = entry.score;
            }
        }
        self.list.remove(best_index)
    }

    fn score_noisy(&mut self, td: &ThreadData) {
        let threats = td.board.all_threats();

        if td.board.in_check() {
            for entry in self.list.iter_mut() {
                let mv = entry.mv;
                let pt = td.board.piece_on(mv.from()).piece_type();

                entry.score = 10000 - 1000 * pt as i32;
            }
        } else {
            for entry in self.list.iter_mut() {
                let mv = entry.mv;
                let captured =
                    if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

                entry.score =
                    16 * captured.value() + td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured);
            }
        }
    }

    fn score_quiet(&mut self, td: &ThreadData, ply: isize) {
        let threats = td.board.all_threats();
        let side = td.board.side_to_move();

        let threatened = {
            let pawn_threats = td.board.piece_threats(PieceType::Pawn);
            let minor_threats =
                pawn_threats | td.board.piece_threats(PieceType::Knight) | td.board.piece_threats(PieceType::Bishop);
            let rook_threats = minor_threats | td.board.piece_threats(PieceType::Rook);
            [Bitboard(0), pawn_threats, pawn_threats, minor_threats, rook_threats, Bitboard(0)]
        };

        let escape = [0, 7768, 8218, 13424, 20208, 0];

        // safe squares where we can attack an opponent piece
        let offense = {
            let mut n = Bitboard(0);
            let mut b = Bitboard(0);
            let mut q = Bitboard(0);
            let pawn_offense = pawn_attacks_setwise(td.board.colors(!side), !side) & !threats;

            for square in td.board.colored_pieces(!side, PieceType::Bishop) & !threats {
                n |= knight_attacks(square);
                q |= rook_attacks(square, td.board.occupancies());
            }

            for square in td.board.colored_pieces(!side, PieceType::Rook) {
                n |= knight_attacks(square);
                b |= bishop_attacks(square, td.board.occupancies());

                if !threats.contains(square) {
                    q |= bishop_attacks(square, td.board.occupancies());
                }
            }
            for square in td.board.colored_pieces(!side, PieceType::Queen) {
                n |= knight_attacks(square);
            }

            [pawn_offense, n & !threats, b & !threats, Bitboard(0), q & !threats, Bitboard(0)]
        };

        // King ring diag attacks and ortho attacks
        let king_ring_ortho = {
            let mut king_ring_ortho = Bitboard(0);
            for square in king_attacks(td.board.king_square(!side)) {
                king_ring_ortho |= rook_attacks(square, td.board.occupancies());
            }
            king_ring_ortho &= !threats;
            king_ring_ortho
        };

        // don't move king wall pawns
        let wall_pawns = if Bitboard::HOME_ROWS[side].contains(td.board.king_square(side)) {
            king_attacks(td.board.king_square(side)) & td.board.pieces(PieceType::Pawn)
        } else {
            Bitboard(0)
        };

        for entry in self.list.iter_mut() {
            let mv = entry.mv;
            let pt = td.board.piece_on(mv.from()).piece_type();

            entry.score = 2048 * td.quiet_history.get(threats, side, mv) / 1024
                + 1536 * td.conthist(ply, 1, mv) / 1024
                + td.conthist(ply, 2, mv)
                + td.conthist(ply, 4, mv)
                + td.conthist(ply, 6, mv)
                + escape[pt] * threatened[pt].contains(mv.from()) as i32
                + 9325 * td.board.checking_squares(pt).contains(mv.to()) as i32
                - 7584 * threatened[pt].contains(mv.to()) as i32
                + 6158 * offense[pt].contains(mv.to()) as i32
                + 5000 * (pt == PieceType::Rook && king_ring_ortho.contains(mv.to())) as i32
                - 4000 * wall_pawns.contains(mv.from()) as i32;
        }
    }
}
