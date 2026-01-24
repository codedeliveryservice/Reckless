use crate::{
    lookup::{
        attacks, between, bishop_attacks, cuckoo, cuckoo_a, cuckoo_b, h1, h2, king_attacks, knight_attacks,
        pawn_attacks, pawn_attacks_setwise, queen_attacks, rook_attacks,
    },
    types::{ArrayVec, Bitboard, Castling, CastlingKind, Color, Move, Piece, PieceType, Square, ZOBRIST},
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
    minor_key: u64,
    non_pawn_keys: [u64; Color::NUM],
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    material: i32,
    plies_from_null: i32,
    repetition: i32,
    captured: Option<Piece>,
    recapture_square: Square,
    threats: Bitboard,
    pinned: [Bitboard; Color::NUM],
    checkers: Bitboard,
}

#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Box<ArrayVec<InternalState, 2048>>,
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

    pub fn hash(&self) -> u64 {
        // To mitigate Graph History Interaction (GHI) problems, the hash key is changed
        // every 8 plies to distinguish between positions that would otherwise appear
        // identical to the transposition table.
        self.state.key ^ ZOBRIST.halfmove_clock[(self.state.halfmove_clock.saturating_sub(8) as usize / 8).min(15)]
    }

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }

    pub const fn minor_key(&self) -> u64 {
        self.state.minor_key
    }

    pub const fn non_pawn_key(&self, color: Color) -> u64 {
        self.state.non_pawn_keys[color as usize]
    }

    pub const fn pinned(&self, color: Color) -> Bitboard {
        self.state.pinned[color as usize]
    }

    pub const fn checkers(&self) -> Bitboard {
        self.state.checkers
    }

    pub const fn threats(&self) -> Bitboard {
        self.state.threats
    }

    pub fn prior_threats(&self) -> Bitboard {
        self.state_stack[self.state_stack.len() - 1].threats
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

    pub const fn colors_bbs(&self) -> [Bitboard; Color::NUM] {
        self.colors
    }

    pub const fn pieces_bbs(&self) -> [Bitboard; PieceType::NUM] {
        self.pieces
    }

    pub fn occupancies(&self) -> Bitboard {
        self.colors(Color::White) | self.colors(Color::Black)
    }

    pub fn of(&self, piece_type: PieceType, color: Color) -> Bitboard {
        self.pieces(piece_type) & self.colors(color)
    }

    pub fn us(&self) -> Bitboard {
        self.colors(self.side_to_move)
    }

    pub fn them(&self) -> Bitboard {
        self.colors(!self.side_to_move)
    }

    pub fn our(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.us()
    }

    pub fn their(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.them()
    }

    pub fn king_square(&self, color: Color) -> Square {
        self.of(PieceType::King, color).lsb()
    }

    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    pub fn moved_piece(&self, mv: Move) -> Piece {
        self.mailbox[mv.from()]
    }

    pub fn has_non_pawns(&self) -> bool {
        self.our(PieceType::Pawn) | self.our(PieceType::King) != self.us()
    }

    pub fn advance_fullmove_counter(&mut self) {
        if self.side_to_move == Color::Black {
            self.fullmove_number += 1;
        }
    }

    pub fn set_frc(&mut self, frc: bool) {
        self.frc = frc;
    }

    pub fn add_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.colors[piece.piece_color()].set(square);
        self.pieces[piece.piece_type()].set(square);
    }

    pub fn remove_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.colors[piece.piece_color()].clear(square);
        self.pieces[piece.piece_type()].clear(square);
    }

    pub fn update_hash(&mut self, piece: Piece, square: Square) {
        let key = ZOBRIST.pieces[piece][square];

        self.state.key ^= key;

        if piece.piece_type() == PieceType::Pawn {
            self.state.pawn_key ^= key;
        } else {
            self.state.non_pawn_keys[piece.piece_color()] ^= key;

            if [PieceType::Knight, PieceType::Bishop, PieceType::King].contains(&piece.piece_type()) {
                self.state.minor_key ^= key;
            }
        }
    }

    /// Checks if the position is a known draw by the fifty-move rule or repetition.
    pub fn is_draw(&self, ply: isize) -> bool {
        self.draw_by_fifty_move_rule() || self.draw_by_repetition(ply as i32)
    }

    /// Checks if the position has repeated once earlier but strictly
    /// after the root, or repeated twice before or at the root.
    pub const fn draw_by_repetition(&self, ply: i32) -> bool {
        self.state.repetition != 0 && self.state.repetition < ply
    }

    pub fn draw_by_fifty_move_rule(&self) -> bool {
        self.state.halfmove_clock >= 100 && (!self.in_check() || self.has_legal_moves())
    }

    /// Checks if the current position has a move that leads to a draw by repetition.
    ///
    /// This method uses a cuckoo hashing algorithm as described in M. N. J. van Kervinck's
    /// paper to detect cycles one ply before they appear in the search of a game tree.
    ///
    /// <http://web.archive.org/web/20201107002606/https://marcelk.net/2013-04-06/paper/upcoming-rep-v2.pdf>
    pub fn upcoming_repetition(&self, ply: usize) -> bool {
        let hm = (self.state.halfmove_clock as usize).min(self.state.plies_from_null as usize);
        if hm < 3 {
            return false;
        }

        let s = |v: usize| self.state_stack[self.state_stack.len() - v].key;
        let s0 = self.state.key;

        let mut other = s0 ^ s(1) ^ ZOBRIST.side;

        for d in (3..=hm).step_by(2) {
            other ^= s(d - 1) ^ s(d) ^ ZOBRIST.side;

            if other != 0 {
                continue;
            }

            let diff = s0 ^ s(d);
            let mut i = h1(diff);

            if cuckoo(i) != diff {
                i = h2(diff);

                if cuckoo(i) != diff {
                    continue;
                }
            }

            if (between(cuckoo_a(i), cuckoo_b(i)) & self.occupancies()).is_empty() {
                if ply > d {
                    return true;
                }

                if self.state.repetition != 0 {
                    return true;
                }
            }
        }

        false
    }

    pub fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        rook_attacks(square, occupancies) & (self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen))
            | bishop_attacks(square, occupancies) & (self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen))
            | pawn_attacks(square, Color::White) & self.of(PieceType::Pawn, Color::Black)
            | pawn_attacks(square, Color::Black) & self.of(PieceType::Pawn, Color::White)
            | knight_attacks(square) & self.pieces(PieceType::Knight)
            | king_attacks(square) & self.pieces(PieceType::King)
    }

    /// Checks if the given move is legal in the current position.
    ///
    /// This method assumes the move has been validated as pseudo-legal
    /// per `Board::is_pseudo_legal`.
    pub fn is_legal(&self, mv: Move) -> bool {
        let from = mv.from();
        let to = mv.to();

        let king = self.our(PieceType::King).lsb();

        if mv.is_en_passant() {
            let occupancies = self.occupancies() ^ from.to_bb() ^ to.to_bb() ^ (to ^ 8).to_bb();

            let diagonal = self.their(PieceType::Bishop) | self.their(PieceType::Queen);
            let orthogonal = self.their(PieceType::Rook) | self.their(PieceType::Queen);

            let diagonal = bishop_attacks(king, occupancies) & diagonal;
            let orthogonal = rook_attacks(king, occupancies) & orthogonal;

            return (orthogonal | diagonal).is_empty();
        }

        if mv.is_castling() {
            let kind = match to {
                Square::G1 => CastlingKind::WhiteKingside,
                Square::C1 => CastlingKind::WhiteQueenside,
                Square::G8 => CastlingKind::BlackKingside,
                Square::C8 => CastlingKind::BlackQueenside,
                _ => unreachable!(),
            };

            return !self.threats().contains(to) && !self.pinned(self.side_to_move).contains(self.castling_rooks[kind]);
        }

        if self.piece_on(from).piece_type() == PieceType::King {
            return !self.threats().contains(to);
        }

        if self.pinned(self.side_to_move).contains(from) {
            let along_pin = between(king, from).contains(to) || between(king, to).contains(from);
            return self.checkers().is_empty() && along_pin;
        }

        if self.checkers().is_multiple() {
            return false;
        }

        if self.checkers().is_empty() {
            return true;
        }

        (self.checkers() | between(king, self.checkers().lsb())).contains(to)
    }

    /// Checks if a move is pseudo-legal in the current position.
    ///
    /// A pseudo-legal move follows the piece's movement rules but does not verify
    /// whether the king is left in check, so it does not guarantee full legality.
    pub fn is_pseudo_legal(&self, mv: Move) -> bool {
        if mv.is_null() {
            return false;
        }

        let from = mv.from();
        let to = mv.to();

        let piece = self.piece_on(from).piece_type();
        let captured = self.piece_on(to).piece_type();

        if mv.is_castling() {
            if !self.us().contains(from) || piece != PieceType::King {
                return false;
            }

            let kind = match to {
                Square::G1 => CastlingKind::WhiteKingside,
                Square::C1 => CastlingKind::WhiteQueenside,
                Square::G8 => CastlingKind::BlackKingside,
                Square::C8 => CastlingKind::BlackQueenside,
                _ => unreachable!(),
            };

            return self.castling().is_allowed(kind)
                && (self.castling_path[kind] & self.occupancies()).is_empty()
                && (self.castling_threat[kind] & self.threats()).is_empty();
        }

        if piece == PieceType::None || !self.us().contains(from) || self.us().contains(to) {
            return false;
        }

        if piece != PieceType::Pawn && (mv.is_double_push() || mv.is_promotion() || mv.is_en_passant()) {
            return false;
        }

        if captured != PieceType::None && (!mv.is_capture() || captured == PieceType::King) {
            return false;
        }

        if mv.is_capture() && !mv.is_en_passant() && !self.them().contains(to) {
            return false;
        }

        if piece == PieceType::Pawn {
            if mv.is_en_passant() {
                return to == self.state.en_passant && pawn_attacks(from, self.side_to_move).contains(to);
            }

            let offset = if self.side_to_move == Color::White { 8 } else { -8 };
            let promotion_rank = if self.side_to_move == Color::White { 7 } else { 0 };

            if mv.is_promotion() != (mv.to().rank() == promotion_rank) {
                return false;
            }

            if mv.is_capture() {
                return pawn_attacks(from, self.side_to_move).contains(to) && self.them().contains(to);
            }

            if mv.is_double_push() {
                return from.rank() == (if self.side_to_move == Color::White { 1 } else { 6 })
                    && from.shift(2 * offset) == to
                    && !self.occupancies().contains(from.shift(offset))
                    && !self.occupancies().contains(to);
            }

            return from.shift(offset) == to && !self.occupancies().contains(to);
        }

        let attacks = match piece {
            PieceType::Knight => knight_attacks(from),
            PieceType::Bishop => bishop_attacks(from, self.occupancies()),
            PieceType::Rook => rook_attacks(from, self.occupancies()),
            PieceType::Queen => queen_attacks(from, self.occupancies()),
            PieceType::King => king_attacks(from),
            _ => unreachable!(),
        };

        attacks.contains(to)
    }

    /// Quickly checks if the move *might* give check to the opponent's king.
    ///
    /// Roughly 90–95% accurate. Does not account for discovered checks, promotions,
    /// en passant, or checks delivered via castling.
    pub fn is_direct_check(&self, mv: Move) -> bool {
        let occupancies = self.occupancies() ^ mv.from().to_bb() ^ mv.to().to_bb();
        let direct_attacks = attacks(self.moved_piece(mv), mv.to(), occupancies);
        direct_attacks.contains(self.their(PieceType::King).lsb())
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
        let occupancies = self.occupancies() ^ self.our(PieceType::King);

        let mut threats = Bitboard::default();

        threats |= pawn_attacks_setwise(self.their(PieceType::Pawn), !self.side_to_move);

        for square in self.their(PieceType::Knight) {
            threats |= knight_attacks(square);
        }

        #[cfg(target_feature = "avx512f")]
        {
            use crate::lookup::slider_attacks_setwise;
            threats |= slider_attacks_setwise(
                self.their(PieceType::Bishop),
                self.their(PieceType::Rook),
                self.their(PieceType::Queen),
                occupancies,
            );
        }

        #[cfg(not(target_feature = "avx512f"))]
        {
            for square in self.their(PieceType::Bishop) | self.their(PieceType::Queen) {
                threats |= bishop_attacks(square, occupancies);
            }

            for square in self.their(PieceType::Rook) | self.their(PieceType::Queen) {
                threats |= rook_attacks(square, occupancies);
            }
        }

        self.state.threats = threats | king_attacks(self.their(PieceType::King).lsb());
    }

    /// Updates the checkers bitboard to mark opponent pieces currently threatening our king,
    /// and our pinned pieces that cannot move without leaving the king in check.
    pub fn update_king_threats(&mut self) {
        let our_king = self.king_square(self.side_to_move);

        self.state.pinned = [Bitboard::default(); 2];
        self.state.checkers = Bitboard::default();

        self.state.checkers |= pawn_attacks(our_king, self.side_to_move) & self.their(PieceType::Pawn);
        self.state.checkers |= knight_attacks(our_king) & self.their(PieceType::Knight);

        let diagonal = self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen);
        let orthogonal = self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen);

        for color in [Color::White, Color::Black] {
            let king = self.king_square(color);

            let diagonal = diagonal & bishop_attacks(king, self.colors(!color)) & self.colors(!color);
            let orthogonal = orthogonal & rook_attacks(king, self.colors(!color)) & self.colors(!color);

            for square in diagonal | orthogonal {
                let blockers = between(king, square) & self.colors(color);
                match blockers.popcount() {
                    0 if color == self.side_to_move => self.state.checkers.set(square),
                    1 => self.state.pinned[color] |= blockers,
                    _ => (),
                }
            }
        }
    }

    pub fn update_hash_keys(&mut self) {
        self.state.key = 0;
        self.state.pawn_key = 0;
        self.state.minor_key = 0;
        self.state.non_pawn_keys = [0; Color::NUM];

        for piece in 0..Piece::NUM {
            let piece = Piece::from_index(piece);

            for square in self.of(piece.piece_type(), piece.piece_color()) {
                self.update_hash(piece, square);
            }
        }

        if self.state.en_passant != Square::None {
            self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
        }

        if self.side_to_move == Color::White {
            self.state.key ^= ZOBRIST.side;
        }

        self.state.key ^= ZOBRIST.castling[self.state.castling];
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

    #[cfg(target_feature = "avx512f")]
    pub unsafe fn mailbox_vector(&self) -> std::arch::x86_64::__m512i {
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
            state_stack: Box::new(ArrayVec::new()),
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

pub struct NullBoardObserver {}

impl BoardObserver for NullBoardObserver {
    fn on_piece_change(&mut self, _: &Board, _: Piece, _: Square, _: bool) {}
    fn on_piece_move(&mut self, _: &Board, _: Piece, _: Square, _: Square) {}
    fn on_piece_mutate(&mut self, _: &Board, _: Piece, _: Piece, _: Square) {}
}
