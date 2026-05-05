use crate::{
    lookup::king_attacks,
    search::NodeType,
    setwise::{bishop_attacks_setwise, knight_attacks_setwise, pawn_attacks_setwise, rook_attacks_setwise},
    thread::ThreadData,
    types::{ArrayVec, Bitboard, Color, MAX_MOVES, Move, MoveEntry, MoveList, PieceType, Square},
};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    GenerateNoisy,
    GoodNoisy,
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
                self.stage = Stage::Quiet;
                td.board.append_quiet_moves(&mut self.list);
                self.score_quiet(td, ply);
            }
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

        for entry in self.list.iter_mut() {
            let mv = entry.mv;
            let captured = td.board.type_on(mv.capture_sq());
            let pt = td.board.type_on(mv.from());

            entry.score = 16 * captured.value()
                + td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured)
                + 4000 * (mv.is_promotion() && mv.promo_piece_type() == PieceType::Queen) as i32
                + (200000 - 20000 * pt as i32) * td.board.in_check() as i32;
        }
    }

    fn score_quiet(&mut self, td: &ThreadData, ply: isize) {
        let threats = td.board.all_threats();
        let side = td.board.side_to_move();
        let occupancies = td.board.occupancies();
        let pawn_threats = td.board.piece_threats(PieceType::Pawn);

        let threatened = {
            let minor_threats =
                pawn_threats | td.board.piece_threats(PieceType::Knight) | td.board.piece_threats(PieceType::Bishop);
            let rook_threats = minor_threats | td.board.piece_threats(PieceType::Rook);
            [Bitboard(0), pawn_threats, pawn_threats, minor_threats, rook_threats, Bitboard(0)]
        };

        let escape = [0, 7768, 8218, 13424, 20208, 0];

        // safe squares where we can attack an opponent piece
        let offense = {
            let knight_vulnerable = (td.board.colored_pieces(!side, PieceType::Bishop) & !threats)
                | td.board.colored_pieces(!side, PieceType::Rook)
                | td.board.colored_pieces(!side, PieceType::Queen);
            let bishop_vulnerable = td.board.colored_pieces(!side, PieceType::Rook);
            let queen_orth_vulnerable = td.board.colored_pieces(!side, PieceType::Bishop) & !threats;
            let queen_diag_vulnerable = td.board.colored_pieces(!side, PieceType::Rook) & !threats;

            let p = pawn_attacks_setwise(td.board.colors(!side), !side) & !threats;
            let n = knight_attacks_setwise(knight_vulnerable) & !threats;
            let b = bishop_attacks_setwise(bishop_vulnerable, occupancies) & !threats;
            let r = Bitboard::file(td.board.king_square(!side).file()) & !threats;
            let q = (rook_attacks_setwise(queen_orth_vulnerable, occupancies)
                | bishop_attacks_setwise(queen_diag_vulnerable, occupancies))
                & !threats;

            [p, n, b, r, q, Bitboard(0)]
        };

        // don't move king wall pawns
        let my_king = td.board.king_square(side);
        let wall_pawns = if Bitboard::HOME_ROWS[side].contains(my_king) {
            king_attacks(my_king) & td.board.pieces(PieceType::Pawn)
        } else {
            Bitboard(0)
        };

        // passed pawns
        let mut passed_space = td.board.colored_pieces(!side, PieceType::Pawn) | pawn_threats;
        passed_space |= passed_space.shift(Square::UP[!side]);
        passed_space |= passed_space.shift(2 * Square::UP[!side]);
        passed_space |= passed_space.shift(4 * Square::UP[!side]);
        passed_space = !passed_space;
        let passed_pawns = td.board.colored_pieces(side, PieceType::Pawn) & passed_space;

        for entry in self.list.iter_mut() {
            let mv = entry.mv;
            let pt = td.board.type_on(mv.from());

            entry.score = 2048 * td.quiet_history.get(threats, side, mv) / 1024
                + 1536 * td.conthist(ply, 1, mv) / 1024
                + td.conthist(ply, 2, mv)
                + td.conthist(ply, 4, mv)
                + td.conthist(ply, 6, mv)
                + escape[pt] * threatened[pt].contains(mv.from()) as i32
                + 9325 * td.board.checking_squares(pt).contains(mv.to()) as i32
                - 7584 * threatened[pt].contains(mv.to()) as i32
                + 5000 * offense[pt].contains(mv.to()) as i32
                - 4000 * wall_pawns.contains(mv.from()) as i32;

            if td.board.material() < 2000 && !passed_pawns.is_empty() && pt == PieceType::King {
                let passed_pawn = if side == Color::White { passed_pawns.msb() } else { passed_pawns.lsb() };
                if mv.to().distance_from(passed_pawn) < mv.from().distance_from(passed_pawn) {
                    entry.score += 3000;
                }
            }
        }
    }
}
