use self::{parser::ParseFenError, zobrist::ZOBRIST};
use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks},
    nnue::Network,
    types::{Bitboard, Castling, Color, Piece, PieceType, Square},
};

#[cfg(test)]
mod tests;

mod makemove;
mod movegen;
mod parser;
mod see;
mod zobrist;

const MAX_PHASE: i32 = 62;
const PHASE_WEIGHTS: [i32; PieceType::NUM - 1] = [0, 3, 3, 5, 9];

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Copy, Clone, Default)]
struct InternalState {
    hash_key: u64,
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    captured: Option<Piece>,
    threats: Bitboard,
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Vec<InternalState>,
    nnue: Network,
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

    /// Returns the Zobrist hash key for the current position.
    pub const fn hash(&self) -> u64 {
        self.state.hash_key
    }

    /// Returns a `Bitboard` for the specified `Color`.
    pub fn colors(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    pub fn pieces(&self, piece_type: PieceType) -> Bitboard {
        self.pieces[piece_type]
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

    /// Finds a piece on the specified square, if found; otherwise, `Piece::None`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    /// Returns `true` if the current side to move has non-pawn material.
    ///
    /// This method is used to minimize the risk of zugzwang when considering the Null Move Heuristic.
    pub fn has_non_pawns(&self) -> bool {
        self.our(PieceType::Pawn) | self.our(PieceType::King) != self.us()
    }

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece<const NNUE: bool>(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.colors[piece.piece_color()].set(square);
        self.pieces[piece.piece_type()].set(square);
        self.update_hash(piece, square);

        if NNUE {
            self.nnue.activate(piece, square);
        }
    }

    /// Removes a piece of the specified type and color from the square.
    pub fn remove_piece<const NNUE: bool>(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.colors[piece.piece_color()].clear(square);
        self.pieces[piece.piece_type()].clear(square);
        self.update_hash(piece, square);

        if NNUE {
            self.nnue.deactivate(piece, square);
        }
    }

    pub fn update_hash(&mut self, piece: Piece, square: Square) {
        self.state.hash_key ^= ZOBRIST.pieces[piece][square];
    }

    /// Calculates the score of the current position from the perspective of the side to move.
    pub fn evaluate(&self) -> i32 {
        let mut eval = self.nnue.evaluate(self.side_to_move);

        #[cfg(not(feature = "datagen"))]
        {
            // Linearly damp the evaluation from 100% to 80% as the game approaches the endgame
            eval -= eval * (MAX_PHASE - self.game_phase()) / (5 * MAX_PHASE);
        }

        eval.clamp(-16384, 16384)
    }

    pub fn game_phase(&self) -> i32 {
        [PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
            .iter()
            .map(|&piece| self.pieces(piece).len() as i32 * PHASE_WEIGHTS[piece])
            .sum::<i32>()
            .min(MAX_PHASE)
    }

    /// Returns `true` if the current position is a known draw by the fifty-move rule or repetition.
    pub fn is_draw(&self) -> bool {
        self.draw_by_repetition() || self.draw_by_fifty_move_rule() || self.draw_by_insufficient_material()
    }

    /// Returns `true` if the current position has already been present at least once
    /// in the board's history.
    ///
    /// This method does not count the number of encounters.
    pub fn draw_by_repetition(&self) -> bool {
        self.state_stack
            .iter()
            .rev()
            .skip(1)
            .step_by(2)
            .take(self.state.halfmove_clock as usize + 1)
            .any(|state| state.hash_key == self.state.hash_key)
    }

    /// Returns `true` if the current position is a known draw by insufficient material:
    /// - Two kings only
    /// - Two kings and one minor piece
    pub fn draw_by_insufficient_material(&self) -> bool {
        match self.occupancies().len() {
            2 => true,
            3 => self.pieces(PieceType::Knight) | self.pieces(PieceType::Bishop) != Bitboard(0),
            _ => false,
        }
    }

    /// Returns `true` if the position is a draw by the fifty-move rule.
    pub const fn draw_by_fifty_move_rule(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    /// Returns `true` if the square is attacked by pieces of the specified color.
    pub fn is_square_attacked_by(&self, square: Square, color: Color) -> bool {
        !(self.attackers_to(square, self.occupancies()) & self.colors(color)).is_empty()
    }

    pub fn in_check(&self) -> bool {
        self.is_threatened(self.our(PieceType::King).lsb())
    }

    pub fn is_threatened(&self, square: Square) -> bool {
        self.state.threats.contains(square)
    }

    pub fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        king_attacks(square) & self.pieces(PieceType::King)
            | knight_attacks(square) & self.pieces(PieceType::Knight)
            | pawn_attacks(square, Color::White) & self.of(PieceType::Pawn, Color::Black)
            | pawn_attacks(square, Color::Black) & self.of(PieceType::Pawn, Color::White)
            | rook_attacks(square, occupancies) & (self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen))
            | bishop_attacks(square, occupancies) & (self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen))
    }

    pub fn generate_threats(&self) -> Bitboard {
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

        threats |= king_attacks(self.their(PieceType::King).lsb());

        threats
    }

    /// Performs Zobrist hashing on `self`, generating an *almost* unique
    /// position hash key from scratch.
    ///
    /// This method should only be used for the initial hash key generation.
    /// For further reference, use `hash_key()` to get a key that is
    /// incrementally updated during the game.
    pub fn generate_hash_key(&self) -> u64 {
        let mut hash = 0;

        for piece in 0..Piece::NUM {
            let piece = Piece::from_index(piece);
            for square in self.of(piece.piece_type(), piece.piece_color()) {
                hash ^= ZOBRIST.pieces[piece][square];
            }
        }

        if self.state.en_passant != Square::None {
            hash ^= ZOBRIST.en_passant[self.state.en_passant];
        }
        if self.side_to_move == Color::White {
            hash ^= ZOBRIST.side;
        }

        hash ^= ZOBRIST.castling[self.state.castling];
        hash
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
            state_stack: Vec::default(),
            nnue: Network::default(),
        }
    }
}
