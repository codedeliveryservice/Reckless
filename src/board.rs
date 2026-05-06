use crate::{
    lookup::{
        attacks, between, bishop_attacks, cuckoo, cuckoo_a, cuckoo_b, h1, h2, king_attacks, knight_attacks,
        pawn_attacks, ray_pass, rook_attacks,
    },
    setwise::{bishop_attacks_setwise, knight_attacks_setwise, pawn_attacks_setwise, rook_attacks_setwise},
    types::{
        Bitboard, Castling, CastlingKind, Color, File, Move, PAWN_HOME_RANK, PROMO_RANK, Piece, PieceType, Square,
        ZOBRIST,
    },
};

#[cfg(test)]
mod tests;

mod makemove;
mod movegen;
mod parser;
mod see;

/// Captures essential information needed to efficiently revert the board to
/// a previous position after making a move.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Copy, Clone, Default)]
struct InternalState {
    key: u64,
    pawn_key: u64,
    non_pawn_keys: [u64; Color::NUM],
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    material: i32,
    plies_from_null: usize,
    repetition: i32,
    captured: Option<Piece>,
    recapture_square: Square,
    piece_threats: [Bitboard; PieceType::NUM],
    all_threats: Bitboard,
    pinned: [Bitboard; Color::NUM],
    pinners: [Bitboard; Color::NUM],
    checkers: Bitboard,
    checking_squares: [Bitboard; PieceType::NUM],
}

#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Vec<InternalState>,
    fullmove_number: usize,
    castling_rights: [u8; Square::NUM],
    castling_path: [Bitboard; 16],
    castling_threat: [Bitboard; 16],
    castling_rooks: [Square; 16],
    frc: bool,
}

impl Board {
    pub fn starting_position() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub const fn is_frc(&self) -> bool {
        self.frc
    }

    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub const fn fullmove_number(&self) -> usize {
        self.fullmove_number
    }

    pub fn halfmove_clock_bucket(&self) -> usize {
        (self.halfmove_clock().saturating_sub(8) as usize / 8).min(15)
    }

    pub fn hash(&self) -> u64 {
        // To mitigate Graph History Interaction (GHI) problems, the hash key is changed
        // every 8 plies to distinguish between positions that would otherwise appear
        // identical to the transposition table.
        self.state.key ^ ZOBRIST.halfmove_clock[self.halfmove_clock_bucket()]
    }

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }

    pub const fn non_pawn_key(&self, color: Color) -> u64 {
        self.state.non_pawn_keys[color as usize]
    }

    pub const fn pinned(&self, color: Color) -> Bitboard {
        self.state.pinned[color as usize]
    }

    pub const fn pinners(&self, color: Color) -> Bitboard {
        self.state.pinners[color as usize]
    }

    pub const fn checking_squares(&self, pt: PieceType) -> Bitboard {
        self.state.checking_squares[pt as usize]
    }

    pub const fn checkers(&self) -> Bitboard {
        self.state.checkers
    }

    pub const fn all_threats(&self) -> Bitboard {
        self.state.all_threats
    }

    pub const fn piece_threats(&self, pt: PieceType) -> Bitboard {
        self.state.piece_threats[pt as usize]
    }

    pub fn prior_threats(&self) -> Bitboard {
        debug_assert!(!self.state_stack.is_empty());
        self.state_stack[self.state_stack.len() - 1].all_threats
    }

    pub const fn captured_piece(&self) -> Option<Piece> {
        self.state.captured
    }

    pub const fn recapture_square(&self) -> Square {
        self.state.recapture_square
    }

    pub const fn en_passant(&self) -> Square {
        self.state.en_passant
    }

    pub const fn castling(&self) -> Castling {
        self.state.castling
    }

    pub const fn halfmove_clock(&self) -> u8 {
        self.state.halfmove_clock
    }

    pub const fn material(&self) -> i32 {
        self.state.material
    }

    pub const fn in_check(&self) -> bool {
        !self.state.checkers.is_empty()
    }

    pub fn colors(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    pub fn pieces(&self, piece_type: PieceType) -> Bitboard {
        self.pieces[piece_type]
    }

    pub fn pieces2(&self, piece_type1: PieceType, piece_type2: PieceType) -> Bitboard {
        self.pieces[piece_type1] | self.pieces[piece_type2]
    }

    pub const fn colors_bbs(&self) -> [Bitboard; Color::NUM] {
        self.colors
    }

    pub const fn pieces_bbs(&self) -> [Bitboard; PieceType::NUM] {
        self.pieces
    }

    pub fn occupancies(&self) -> Bitboard {
        self.colors(Color::White) | self.colors(Color::Black)
    }

    pub fn colored_pieces(&self, side: Color, piece_type: PieceType) -> Bitboard {
        self.colors(side) & self.pieces(piece_type)
    }

    pub fn colored_pieces2(&self, side: Color, pt1: PieceType, pt2: PieceType) -> Bitboard {
        self.colors(side) & (self.pieces(pt1) | self.pieces(pt2))
    }

    pub fn king_square(&self, color: Color) -> Square {
        self.colored_pieces(color, PieceType::King).lsb()
    }

    pub fn type_on(&self, square: Square) -> PieceType {
        self.mailbox[square].piece_type()
    }

    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    pub fn moved_piece(&self, mv: Move) -> Piece {
        self.mailbox[mv.from()]
    }

    pub const fn advance_fullmove_counter(&mut self) {
        self.fullmove_number += self.side_to_move() as usize;
    }

    pub const fn set_frc(&mut self, frc: bool) {
        self.frc = frc;
    }

    pub fn add_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.colors[piece.color()].set(square);
        self.pieces[piece.piece_type()].set(square);
    }

    pub fn remove_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.colors[piece.color()].clear(square);
        self.pieces[piece.piece_type()].clear(square);
    }

    pub fn update_hash(&mut self, piece: Piece, square: Square) {
        let key = ZOBRIST.pieces[piece][square];

        self.state.key ^= key;

        if piece.piece_type() == PieceType::Pawn {
            self.state.pawn_key ^= key;
        } else {
            self.state.non_pawn_keys[piece.color()] ^= key;
        }
    }

    /// Checks for a material draw
    pub fn draw_by_material(&self) -> bool {
        let stm = self.side_to_move();
        if (self.pieces(PieceType::Pawn) | self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen)) != Bitboard(0)
        {
            return false;
        }

        let piece_count = self.occupancies().popcount();
        if piece_count != 4 {
            return piece_count < 4;
        }

        // Here on, there are exactly 2 non-king minors
        if self.colored_pieces2(stm, PieceType::Bishop, PieceType::Knight).popcount() == 1 {
            return true;
        }

        if self.pieces(PieceType::Knight) != Bitboard(0) {
            return false;
        }

        (self.pieces(PieceType::Bishop) & Bitboard::LIGHT_SQUARES).popcount() != 1
    }

    /// Checks if the position has repeated once earlier but strictly
    /// after the root, or repeated twice before or at the root.
    pub const fn draw_by_repetition(&self, ply: i32) -> bool {
        self.state.repetition != 0 && self.state.repetition < ply
    }

    pub fn draw_by_fifty_move_rule(&self) -> bool {
        self.halfmove_clock() >= 100 && (!self.in_check() || self.has_legal_moves())
    }

    /// Checks if the position is a known draw by material, fifty-move or repetition.
    pub fn is_draw(&self, ply: isize) -> bool {
        self.draw_by_material() || self.draw_by_fifty_move_rule() || self.draw_by_repetition(ply as i32)
    }

    /// Checks if the current position has a move that leads to a draw by repetition.
    ///
    /// This method uses a cuckoo hashing algorithm as described in M. N. J. van Kervinck's
    /// paper to detect cycles one ply before they appear in the search of a game tree.
    ///
    /// <http://web.archive.org/web/20201107002606/https://marcelk.net/2013-04-06/paper/upcoming-rep-v2.pdf>
    pub fn upcoming_repetition(&self, ply: usize) -> bool {
        let half_moves = self.state.plies_from_null.min(self.state.halfmove_clock as usize);
        if half_moves < 3 {
            return false;
        }

        let current_key = self.state.key;
        let stack = &self.state_stack;
        let len = stack.len();

        let mut index = len - 1;
        let mut other = current_key ^ stack[index].key ^ ZOBRIST.side;

        for compared_ply in (3..=half_moves).step_by(2) {
            index -= 1;
            other ^= stack[index].key ^ stack[index - 1].key ^ ZOBRIST.side;
            index -= 1;

            if other != 0 {
                continue;
            }

            let diff = current_key ^ stack[index].key;
            let mut cuckoo_index = h1(diff);

            if cuckoo(cuckoo_index) != diff {
                cuckoo_index = h2(diff);
                if cuckoo(cuckoo_index) != diff {
                    continue;
                }
            }

            if (between(cuckoo_a(cuckoo_index), cuckoo_b(cuckoo_index)) & self.occupancies()).is_empty()
                && (ply > compared_ply || stack[index].repetition != 0)
            {
                return true;
            }
        }

        false
    }

    pub fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        (rook_attacks(square, occupancies) & self.pieces2(PieceType::Rook, PieceType::Queen))
            | (bishop_attacks(square, occupancies) & self.pieces2(PieceType::Bishop, PieceType::Queen))
            | (pawn_attacks(square, Color::White) & self.colored_pieces(Color::Black, PieceType::Pawn))
            | (pawn_attacks(square, Color::Black) & self.colored_pieces(Color::White, PieceType::Pawn))
            | (knight_attacks(square) & self.pieces(PieceType::Knight))
            | (king_attacks(square) & self.pieces(PieceType::King))
    }

    pub fn is_legal(&self, mv: Move) -> bool {
        debug_assert!(mv.is_present());
        let stm = self.side_to_move();
        let king = self.king_square(stm);
        let from = mv.from();
        let to = mv.to();

        if !self.colors(stm).contains(from) {
            return false;
        }

        let piece = self.piece_on(from);

        if piece.piece_type() == PieceType::King {
            if mv.is_castling() {
                let kind = CastlingKind::KINDS[stm][(to.file() == File::G) as usize];

                return self.castling().is_allowed(kind)
                    && (self.castling_path[kind] & self.occupancies()).is_empty()
                    && (self.castling_threat[kind] & self.all_threats()).is_empty()
                    && !self.pinned(stm).contains(self.castling_rooks[kind]);
            }

            return !mv.is_special()
                && !self.colors(stm).contains(to)
                && (mv.is_capture() == self.colors(!stm).contains(to))
                && (king_attacks(from) & !self.all_threats()).contains(to);
        }

        if self.colors(stm).contains(to)
            || (self.pinned(stm).contains(from) && !ray_pass(king, from).contains(to))
            || (self.in_check()
                && (self.checkers().is_multiple()
                    || (!mv.is_en_passant() && !(self.checkers() | between(king, self.checkers().lsb())).contains(to))))
        {
            return false;
        }

        if piece.piece_type() == PieceType::Pawn {
            if mv.is_en_passant() {
                let occupancies = self.occupancies() ^ from.to_bb() ^ to.to_bb() ^ (to ^ 8).to_bb();
                let diagonal = self.colored_pieces2(!stm, PieceType::Bishop, PieceType::Queen);
                let orthogonal = self.colored_pieces2(!stm, PieceType::Rook, PieceType::Queen);
                let diagonal = bishop_attacks(king, occupancies) & diagonal;
                let orthogonal = rook_attacks(king, occupancies) & orthogonal;
                return to == self.en_passant()
                    && pawn_attacks(from, stm).contains(to)
                    && (orthogonal | diagonal).is_empty();
            }

            if mv.is_promotion() != (mv.to().rank() == PROMO_RANK[stm]) {
                return false;
            }

            if mv.is_capture() {
                return pawn_attacks(from, stm).contains(to) && self.colors(!stm).contains(to);
            }

            if mv.is_double_push() {
                return from.rank() == PAWN_HOME_RANK[stm]
                    && from.shift(2 * Square::UP[stm]) == to
                    && !self.occupancies().contains(from.shift(Square::UP[stm]))
                    && !self.occupancies().contains(to);
            }

            return !mv.is_castling() && from.shift(Square::UP[stm]) == to && !self.occupancies().contains(to);
        }

        !mv.is_special()
            && (mv.is_capture() == self.colors(!stm).contains(to))
            && attacks(piece, from, self.occupancies()).contains(to)
    }

    /// Quickly checks if the move *might* give check to the opponent's king.
    ///
    /// Roughly 90–95% accurate. Does not account for discovered checks, promotions,
    /// en passant, or checks delivered via castling.
    pub fn is_direct_check(&self, mv: Move) -> bool {
        self.checking_squares(self.moved_piece(mv).piece_type()).contains(mv.to())
    }

    pub fn update_threats(&mut self) {
        // The king is excluded from the occupancy bitboard when computing threats,
        // letting sliders "see through" it as if the king weren't blocking their path.
        //
        // Although this changes the resulting threat bitboard, it has no impact on
        // engine behavior, since such squares are not legal move targets, so threat
        // history remains unaffected by this change.
        //
        // This "hack" is used to speed up the implementation of `Board::is_legal`.
        let stm = self.side_to_move();
        let occupancies = self.occupancies() ^ self.colored_pieces(stm, PieceType::King);

        self.state.piece_threats[PieceType::Pawn] =
            pawn_attacks_setwise(self.colored_pieces(!stm, PieceType::Pawn), !stm);
        self.state.piece_threats[PieceType::Knight] =
            knight_attacks_setwise(self.colored_pieces(!stm, PieceType::Knight));
        self.state.piece_threats[PieceType::Bishop] =
            bishop_attacks_setwise(self.colored_pieces(!stm, PieceType::Bishop), occupancies);
        self.state.piece_threats[PieceType::Rook] =
            rook_attacks_setwise(self.colored_pieces(!stm, PieceType::Rook), occupancies);
        self.state.piece_threats[PieceType::Queen] =
            bishop_attacks_setwise(self.colored_pieces(!stm, PieceType::Queen), occupancies)
                | rook_attacks_setwise(self.colored_pieces(!stm, PieceType::Queen), occupancies);
        self.state.piece_threats[PieceType::King] = king_attacks(self.king_square(!stm));

        self.state.all_threats = self.piece_threats(PieceType::Pawn)
            | self.piece_threats(PieceType::Knight)
            | self.piece_threats(PieceType::Bishop)
            | self.piece_threats(PieceType::Rook)
            | self.piece_threats(PieceType::Queen)
            | self.piece_threats(PieceType::King);

        let diagonal = self.pieces2(PieceType::Bishop, PieceType::Queen);
        let orthogonal = self.pieces2(PieceType::Rook, PieceType::Queen);

        self.state.pinned = [Bitboard::default(); 2];
        self.state.pinners = [Bitboard::default(); 2];

        for color in [Color::White, Color::Black] {
            let king = self.king_square(color);

            if color == stm {
                self.state.checkers = (pawn_attacks(king, stm) & self.colored_pieces(!stm, PieceType::Pawn))
                    | (knight_attacks(king) & self.colored_pieces(!stm, PieceType::Knight));
            } else {
                self.state.checking_squares[PieceType::Pawn] = pawn_attacks(king, !stm);
                self.state.checking_squares[PieceType::Knight] = knight_attacks(king);
                self.state.checking_squares[PieceType::Bishop] = bishop_attacks(king, self.occupancies());
                self.state.checking_squares[PieceType::Rook] = rook_attacks(king, self.occupancies());
                self.state.checking_squares[PieceType::Queen] =
                    self.checking_squares(PieceType::Bishop) | self.checking_squares(PieceType::Rook);
            }

            let diagonal = diagonal & bishop_attacks(king, self.colors(!color)) & self.colors(!color);
            let orthogonal = orthogonal & rook_attacks(king, self.colors(!color)) & self.colors(!color);

            for square in diagonal | orthogonal {
                let blockers = between(king, square) & self.colors(color);
                match blockers.popcount() {
                    0 => {
                        debug_assert_eq!(color, stm);
                        self.state.checkers.set(square);
                    }
                    1 => {
                        self.state.pinners[!color].set(square);
                        self.state.pinned[color] |= blockers;
                    }
                    _ => (),
                }
            }
        }
    }

    pub fn update_hash_keys(&mut self) {
        self.state.key = 0;
        self.state.pawn_key = 0;
        self.state.non_pawn_keys = [0; Color::NUM];

        for piece in 0..Piece::NUM {
            let piece = Piece::from_index(piece);

            for square in self.colored_pieces(piece.color(), piece.piece_type()) {
                self.update_hash(piece, square);
            }
        }

        if self.en_passant() != Square::None {
            self.state.key ^= ZOBRIST.en_passant[self.en_passant()];
        }

        if self.side_to_move() == Color::White {
            self.state.key ^= ZOBRIST.side;
        }

        self.state.key ^= ZOBRIST.castling[self.state.castling];
    }

    fn is_en_passant_valid(&self) -> bool {
        let stm = self.side_to_move();
        let king = self.king_square(stm);
        let pushed_pawn = self.en_passant() ^ 8;

        let pawns = pawn_attacks(self.en_passant(), !stm) & self.colored_pieces(stm, PieceType::Pawn);

        for attacker in pawns {
            let occ = self.en_passant().to_bb() | (self.occupancies() ^ pushed_pawn.to_bb() ^ attacker.to_bb());
            let king_attackers = occ & self.attackers_to(king, occ) & self.colors(!stm);

            if king_attackers.is_empty() {
                return true;
            }
        }

        false
    }

    /// We verify is self.state.enpassant is valid, and remove it if it is not.
    /// This must be called after pinners and checkers have been updated.
    fn update_en_passant(&mut self) {
        if self.en_passant() == Square::None {
            return;
        }

        if self.is_en_passant_valid() {
            return;
        }

        self.state.key ^= ZOBRIST.en_passant[self.en_passant()];
        self.state.en_passant = Square::None;
    }

    pub fn get_castling_rook(&self, king_to: Square) -> (Square, Square) {
        match king_to {
            Square::G1 => (self.castling_rooks[CastlingKind::WhiteKingside], Square::F1),
            Square::C1 => (self.castling_rooks[CastlingKind::WhiteQueenside], Square::D1),
            Square::G8 => (self.castling_rooks[CastlingKind::BlackKingside], Square::F8),
            Square::C8 => (self.castling_rooks[CastlingKind::BlackQueenside], Square::D8),
            _ => unreachable!(),
        }
    }

    #[cfg(target_feature = "avx2")]
    pub unsafe fn mailbox_vector_avx2(&self) -> [std::arch::x86_64::__m256i; 2] {
        use std::arch::x86_64::*;
        let ptr: *const __m256i = self.mailbox.as_ptr().cast();
        [_mm256_loadu_si256(ptr), _mm256_loadu_si256(ptr.add(1))]
    }

    #[cfg(target_feature = "avx512f")]
    pub unsafe fn mailbox_vector_avx512(&self) -> std::arch::x86_64::__m512i {
        std::arch::x86_64::_mm512_loadu_si512(self.mailbox.as_ptr().cast())
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            side_to_move: Color::White,
            state: InternalState::default(),
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
            mailbox: [Piece::None; Square::NUM],
            state_stack: Vec::with_capacity(2048),
            fullmove_number: 0,
            castling_rights: [0b1111; Square::NUM],
            castling_path: [Bitboard::default(); 16],
            castling_threat: [Bitboard::default(); 16],
            castling_rooks: [Square::None; 16],
            frc: false,
        }
    }
}

pub trait BoardObserver {
    fn on_piece_change(&mut self, board: &Board, piece: Piece, sq: Square, add: bool);
    fn on_piece_move(&mut self, board: &Board, piece: Piece, from: Square, to: Square);
    fn on_piece_mutate(&mut self, board: &Board, old_piece: Piece, new_piece: Piece, sq: Square);
}

pub struct NullBoardObserver;

impl BoardObserver for NullBoardObserver {
    fn on_piece_change(&mut self, _: &Board, _: Piece, _: Square, _: bool) {}
    fn on_piece_move(&mut self, _: &Board, _: Piece, _: Square, _: Square) {}
    fn on_piece_mutate(&mut self, _: &Board, _: Piece, _: Piece, _: Square) {}
}
