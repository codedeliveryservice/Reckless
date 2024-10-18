use self::{parser::ParseFenError, zobrist::ZOBRIST};
use crate::{
    nnue::Network,
    types::{Bitboard, Castling, Color, FullMove, Move, Piece, Score, Square},
};

#[cfg(test)]
mod tests;

mod makemove;
mod movegen;
mod parser;
mod zobrist;

const MAX_PHASE: i32 = 62;
const PHASE_WEIGHTS: [i32; Piece::NUM - 1] = [0, 3, 3, 5, 9];

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Copy, Clone, Default)]
struct InternalState {
    hash_key: u64,
    pawn_key: u64,
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    captured: Option<Piece>,
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Vec<InternalState>,
    move_stack: Vec<FullMove>,
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

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }

    /// Returns a `Bitboard` for the specified `Color`.
    pub fn colors(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    pub fn pieces(&self, piece: Piece) -> Bitboard {
        self.pieces[piece]
    }

    /// Returns a `Bitboard` for all pieces on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.colors(Color::White) | self.colors(Color::Black)
    }

    /// Returns a `Bitboard` for the specified `Piece` type and `Color`.
    pub fn of(&self, piece: Piece, color: Color) -> Bitboard {
        self.pieces(piece) & self.colors(color)
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
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces(piece) & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces(piece) & self.them()
    }

    /// Finds a piece on the specified square, if found; otherwise, `Piece::None`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    /// Returns `true` if the current side to move has non-pawn material.
    ///
    /// This method is used to minimize the risk of zugzwang when considering the Null Move Heuristic.
    pub fn has_non_pawn_material(&self) -> bool {
        self.our(Piece::Pawn) | self.our(Piece::King) != self.us()
    }

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece<const NNUE: bool>(&mut self, color: Color, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.pieces[piece].set(square);
        self.colors[color].set(square);
        self.update_hash(color, piece, square);
        if NNUE {
            self.nnue.activate(color, piece, square);
        }
    }

    /// Removes a piece of the specified type and color from the square.
    pub fn remove_piece<const NNUE: bool>(&mut self, color: Color, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.pieces[piece].clear(square);
        self.colors[color].clear(square);
        self.update_hash(color, piece, square);
        if NNUE {
            self.nnue.deactivate(color, piece, square);
        }
    }

    pub fn update_hash(&mut self, color: Color, piece: Piece, square: Square) {
        self.state.hash_key ^= ZOBRIST.pieces[color][piece][square];

        if piece == Piece::Pawn {
            self.state.pawn_key ^= ZOBRIST.pieces[color][piece][square];
        }
    }

    /// Calculates the score of the current position from the perspective of the side to move.
    pub fn evaluate(&self) -> i32 {
        let mut eval = self.nnue.evaluate(self.side_to_move);

        #[cfg(not(feature = "datagen"))]
        {
            // Linearly damp the evaluation from 100% to 80% as the game approaches the endgame
            eval -= eval * (MAX_PHASE - self.game_phase()) / (5 * MAX_PHASE);
        }

        // Clamp the evaluation within mate bounds
        eval.clamp(-Score::MATE_BOUND + 1, Score::MATE_BOUND - 1)
    }

    pub fn game_phase(&self) -> i32 {
        [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen]
            .iter()
            .map(|&piece| self.pieces(piece).len() as i32 * PHASE_WEIGHTS[piece])
            .sum::<i32>()
            .min(MAX_PHASE)
    }

    /// Returns `true` if the current position is a known draw by the fifty-move rule or repetition.
    pub fn is_draw(&self) -> bool {
        self.draw_by_repetition() || self.draw_by_fifty_move_rule()
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
    ///
    /// This method is only used for data generation.
    #[cfg(feature = "datagen")]
    pub fn draw_by_insufficient_material(&self) -> bool {
        match self.occupancies().count() {
            2 => true,
            3 => {
                let minors = self.pieces(Piece::Knight) | self.pieces(Piece::Bishop);
                !minors.is_empty()
            }
            _ => false,
        }
    }

    /// Returns `true` if the position is a draw by the fifty-move rule.
    pub const fn draw_by_fifty_move_rule(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    /// Returns the move at the specified index from the tail of the move stack.
    /// E.g. `tail_move(1)` returns the last move made.
    pub fn tail_move(&self, index: usize) -> FullMove {
        match self.move_stack.len().checked_sub(index) {
            Some(index) => self.move_stack[index],
            None => FullMove::NULL,
        }
    }

    /// Returns `true` if the last move made was a null move.
    pub fn is_last_move_null(&self) -> bool {
        self.move_stack.last() == Some(&FullMove::NULL)
    }

    /// Returns `true` if the square is attacked by pieces of the specified color.
    pub fn is_square_attacked_by(&self, square: Square, color: Color) -> bool {
        !(self.attackers_to(square, self.occupancies()) & self.colors(color)).is_empty()
    }

    pub fn is_in_check(&self) -> bool {
        let king = self.our(Piece::King).lsb();
        self.is_square_attacked_by(king, !self.side_to_move)
    }

    pub fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        use crate::lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

        king_attacks(square) & self.pieces(Piece::King)
            | knight_attacks(square) & self.pieces(Piece::Knight)
            | pawn_attacks(square, Color::White) & self.of(Piece::Pawn, Color::Black)
            | pawn_attacks(square, Color::Black) & self.of(Piece::Pawn, Color::White)
            | rook_attacks(square, occupancies) & (self.pieces(Piece::Rook) | self.pieces(Piece::Queen))
            | bishop_attacks(square, occupancies) & (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen))
    }

    /// Estimates the resulting Zobrist hash key after making the move.
    pub fn key_after(&self, mv: Move) -> u64 {
        let piece = self.piece_on(mv.start());
        let start = mv.start();
        let target = mv.target();

        let mut key = self.state.hash_key;

        key ^= ZOBRIST.pieces[self.side_to_move][piece][start];
        key ^= ZOBRIST.pieces[self.side_to_move][piece][target];

        if mv.is_capture() && !mv.is_en_passant() {
            let capture = self.piece_on(target);
            key ^= ZOBRIST.pieces[!self.side_to_move][capture][target];
        }

        key ^= ZOBRIST.side;
        key
    }

    /// Performs Zobrist hashing on `self`, generating an *almost* unique
    /// position hash key from scratch.
    ///
    /// This method should only be used for the initial hash key generation.
    /// For further reference, use `self.hash_key` to get a key that is
    /// incrementally updated during the game due to performance considerations.
    pub fn generate_hash_key(&self) -> u64 {
        let mut hash = 0;

        for piece in 0..Piece::NUM {
            let piece = Piece::new(piece);
            for color in [Color::White, Color::Black] {
                for square in self.of(piece, color) {
                    hash ^= ZOBRIST.pieces[color][piece][square];
                }
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

    pub fn generate_pawn_key(&self) -> u64 {
        let mut hash = 0;
        for color in [Color::White, Color::Black] {
            for square in self.of(Piece::Pawn, color) {
                hash ^= ZOBRIST.pieces[color][Piece::Pawn][square];
            }
        }
        hash
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            side_to_move: Color::White,
            state: InternalState::default(),
            pieces: [Bitboard::default(); Piece::NUM],
            colors: [Bitboard::default(); Color::NUM],
            mailbox: [Piece::None; Square::NUM],
            state_stack: Vec::default(),
            move_stack: Vec::default(),
            nnue: Network::default(),
        }
    }
}
