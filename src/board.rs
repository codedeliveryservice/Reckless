use self::parser::ParseFenError;
use crate::{
    lookup::{
        between, bishop_attacks, cuckoo, cuckoo_a, cuckoo_b, h1, h2, king_attacks, knight_attacks, pawn_attacks,
        queen_attacks, rook_attacks,
    },
    types::{
        ArrayVec, Bitboard, BlackKingSide, BlackQueenSide, Castling, CastlingKind, Color, Move, Piece, PieceType,
        Square, WhiteKingSide, WhiteQueenSide, ZOBRIST,
    },
};

#[cfg(test)]
mod tests;

mod makemove;
mod movegen;
mod parser;
mod see;

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Copy, Clone, Default)]
struct InternalState {
    key: u64,
    pawn_key: u64,
    minor_key: u64,
    major_key: u64,
    non_pawn_keys: [u64; Color::NUM],
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    plies_from_null: i32,
    repetition: i32,
    captured: Option<Piece>,
    threats: Bitboard,
    pinned: Bitboard,
    checkers: Bitboard,
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Box<ArrayVec<InternalState, 2048>>,
    fullmove_number: usize,
}

impl Board {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    pub fn new(fen: &str) -> Result<Self, ParseFenError> {
        fen.parse()
    }

    /// Returns the board corresponding to the starting position.
    pub fn starting_position() -> Self {
        Self::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub const fn fullmove_number(&self) -> usize {
        self.fullmove_number
    }

    /// Returns the Zobrist hash key for the current position.
    pub fn hash(&self) -> u64 {
        self.state.key ^ ZOBRIST.halfmove_clock[(self.state.halfmove_clock.saturating_sub(8) as usize / 8).min(15)]
    }

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }

    pub const fn minor_key(&self) -> u64 {
        self.state.minor_key
    }

    pub const fn major_key(&self) -> u64 {
        self.state.major_key
    }

    pub const fn non_pawn_key(&self, color: Color) -> u64 {
        self.state.non_pawn_keys[color as usize]
    }

    pub const fn pinned(&self) -> Bitboard {
        self.state.pinned
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

    pub const fn en_passant(&self) -> Square {
        self.state.en_passant
    }

    pub const fn castling(&self) -> Castling {
        self.state.castling
    }

    /// Returns a `Bitboard` for the specified `Color`.
    pub fn colors(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    pub fn pieces(&self, piece_type: PieceType) -> Bitboard {
        self.pieces[piece_type]
    }

    pub fn colors_bbs(&self) -> [Bitboard; Color::NUM] {
        self.colors
    }

    pub fn pieces_bbs(&self) -> [Bitboard; PieceType::NUM] {
        self.pieces
    }

    /// Returns a `Bitboard` for all pieces on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.colors(Color::White) | self.colors(Color::Black)
    }

    /// Returns a `Bitboard` for the specified `Piece` type and `Color`.
    pub fn of(&self, piece_type: PieceType, color: Color) -> Bitboard {
        self.pieces(piece_type) & self.colors(color)
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    pub fn us(&self) -> Bitboard {
        self.colors(self.side_to_move)
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    pub fn them(&self) -> Bitboard {
        self.colors(!self.side_to_move)
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    pub fn our(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    pub fn their(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.them()
    }

    pub fn king_square(&self, color: Color) -> Square {
        self.of(PieceType::King, color).lsb()
    }

    /// Finds a piece on the specified square, if found; otherwise, `Piece::None`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    pub fn moved_piece(&self, mv: Move) -> Piece {
        self.mailbox[mv.from()]
    }

    /// Returns `true` if the current side to move has non-pawn material.
    ///
    /// This method is used to minimize the risk of zugzwang when considering the Null Move Heuristic.
    pub fn has_non_pawns(&self) -> bool {
        self.our(PieceType::Pawn) | self.our(PieceType::King) != self.us()
    }

    pub fn increment_game_ply(&mut self) {
        if self.side_to_move == Color::Black {
            self.fullmove_number += 1;
        }
    }

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.colors[piece.piece_color()].set(square);
        self.pieces[piece.piece_type()].set(square);
    }

    /// Removes a piece of the specified type and color from the square.
    pub fn remove_piece(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.colors[piece.piece_color()].clear(square);
        self.pieces[piece.piece_type()].clear(square);
    }

    pub fn update_hash(&mut self, piece: Piece, square: Square) {
        self.state.key ^= ZOBRIST.pieces[piece][square];

        if piece.piece_type() == PieceType::Pawn {
            self.state.pawn_key ^= ZOBRIST.pieces[piece][square];
        } else {
            self.state.non_pawn_keys[piece.piece_color()] ^= ZOBRIST.pieces[piece][square];

            if [PieceType::Knight, PieceType::Bishop].contains(&piece.piece_type()) {
                self.state.minor_key ^= ZOBRIST.pieces[piece][square];
            } else if [PieceType::Rook, PieceType::Queen].contains(&piece.piece_type()) {
                self.state.major_key ^= ZOBRIST.pieces[piece][square];
            } else {
                self.state.minor_key ^= ZOBRIST.pieces[piece][square];
                self.state.major_key ^= ZOBRIST.pieces[piece][square];
            }
        }
    }

    /// Returns `true` if the current position is a known draw by the fifty-move rule or repetition.
    pub fn is_draw(&self, ply: usize) -> bool {
        self.draw_by_fifty_move_rule() || self.draw_by_repetition(ply as i32)
    }

    /// Return a draw score if a position repeats once earlier but strictly
    /// after the root, or repeats twice before or at the root.
    pub const fn draw_by_repetition(&self, ply: i32) -> bool {
        self.state.repetition != 0 && self.state.repetition < ply
    }

    pub fn draw_by_fifty_move_rule(&self) -> bool {
        self.state.halfmove_clock >= 100 && (!self.in_check() || self.has_legal_moves())
    }

    pub const fn halfmove_clock(&self) -> u8 {
        self.state.halfmove_clock
    }

    pub const fn in_check(&self) -> bool {
        !self.state.checkers.is_empty()
    }

    pub const fn is_threatened(&self, square: Square) -> bool {
        self.state.threats.contains(square)
    }

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

        if self.piece_on(from).piece_type() == PieceType::King {
            let attackers = self.attackers_to(to, self.occupancies() ^ from.to_bb()) & self.them();
            return attackers.is_empty();
        }

        if self.pinned().contains(from) {
            let along_pin = between(king, from).contains(to) || between(king, to).contains(from);
            return self.checkers().is_empty() && along_pin;
        }

        if self.checkers().multiple() {
            return false;
        }

        if self.checkers().is_empty() {
            return true;
        }

        (self.checkers() | between(king, self.checkers().lsb())).contains(to)
    }

    pub fn is_pseudo_legal(&self, mv: Move) -> bool {
        if mv.is_null() {
            return false;
        }

        let from = mv.from();
        let to = mv.to();

        let piece = self.piece_on(from).piece_type();
        let captured = self.piece_on(to).piece_type();

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

        if mv.is_castling() {
            macro_rules! check_castling {
                ($kind:tt) => {
                    ($kind::PATH_MASK & self.occupancies()).is_empty()
                        && self.state.castling.is_allowed::<$kind>()
                        && $kind::CHECK_SQUARES.iter().all(|&square| !self.is_threatened(square))
                };
            }

            return piece == PieceType::King
                && match mv {
                    WhiteKingSide::CASTLING_MOVE => check_castling!(WhiteKingSide),
                    WhiteQueenSide::CASTLING_MOVE => check_castling!(WhiteQueenSide),
                    BlackKingSide::CASTLING_MOVE => check_castling!(BlackKingSide),
                    BlackQueenSide::CASTLING_MOVE => check_castling!(BlackQueenSide),
                    _ => unreachable!(),
                };
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
                return from.shift(2 * offset) == to
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

    pub fn update_threats(&mut self) {
        let occupancies = self.occupancies();
        let mut threats = Bitboard::default();

        for square in self.their(PieceType::Pawn) {
            threats |= pawn_attacks(square, !self.side_to_move);
        }

        for square in self.their(PieceType::Knight) {
            threats |= knight_attacks(square);
        }

        for square in self.their(PieceType::Bishop) | self.their(PieceType::Queen) {
            threats |= bishop_attacks(square, occupancies);
        }

        for square in self.their(PieceType::Rook) | self.their(PieceType::Queen) {
            threats |= rook_attacks(square, occupancies);
        }

        self.state.threats = threats | king_attacks(self.their(PieceType::King).lsb());
    }

    pub fn update_king_threats(&mut self) {
        let king = self.our(PieceType::King).lsb();

        self.state.pinned = Bitboard::default();
        self.state.checkers = Bitboard::default();

        self.state.checkers |= pawn_attacks(king, self.side_to_move) & self.their(PieceType::Pawn);
        self.state.checkers |= knight_attacks(king) & self.their(PieceType::Knight);

        let diagonal = self.their(PieceType::Bishop) | self.their(PieceType::Queen);
        let orthogonal = self.their(PieceType::Rook) | self.their(PieceType::Queen);

        let diagonal = bishop_attacks(king, self.them()) & diagonal;
        let orthogonal = rook_attacks(king, self.them()) & orthogonal;

        for square in diagonal | orthogonal {
            let blockers = between(king, square) & self.us();
            match blockers.len() {
                0 => self.state.checkers.set(square),
                1 => self.state.pinned |= blockers,
                _ => (),
            }
        }
    }

    pub fn update_hash_keys(&mut self) {
        self.state.key = 0;
        self.state.pawn_key = 0;
        self.state.minor_key = 0;
        self.state.major_key = 0;
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

    pub const fn get_castling_rook(king_to: Square) -> (Square, Square) {
        match king_to {
            Square::G1 => (Square::H1, Square::F1),
            Square::C1 => (Square::A1, Square::D1),
            Square::G8 => (Square::H8, Square::F8),
            Square::C8 => (Square::A8, Square::D8),
            _ => unreachable!(),
        }
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
        }
    }
}
