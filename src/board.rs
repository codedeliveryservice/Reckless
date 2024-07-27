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

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Default, Clone, Copy)]
struct InternalState {
    hash: u64,
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Clone)]
pub struct Board {
    pub side_to_move: Color,
    pub ply: usize,
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

    /// Returns the Zobrist hash key for the current position.
    pub const fn hash(&self) -> u64 {
        self.state.hash
    }

    /// Returns a `Bitboard` for the specified `Color`.
    pub fn colors(&self, color: Color) -> Bitboard {
        self.state.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    pub fn pieces(&self, piece: Piece) -> Bitboard {
        self.state.pieces[piece]
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
        for index in 0..Piece::NUM {
            if self.state.pieces[index].contains(square) {
                return Piece::new(index);
            }
        }
        Piece::None
    }

    /// Returns `true` if the current side to move has non-pawn material.
    ///
    /// This method is used to minimize the risk of zugzwang when considering the Null Move Heuristic.
    pub fn has_non_pawn_material(&self) -> bool {
        self.our(Piece::Pawn) | self.our(Piece::King) != self.us()
    }

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece<const UPDATE_NNUE: bool>(&mut self, piece: Piece, color: Color, square: Square) {
        self.state.pieces[piece].set(square);
        self.state.colors[color].set(square);
        self.state.hash ^= ZOBRIST.pieces[color][piece][square];
        if UPDATE_NNUE {
            self.nnue.activate(color, piece, square);
        }
    }

    /// Removes a piece of the specified type and color from the square.
    pub fn remove_piece<const UPDATE_NNUE: bool>(&mut self, piece: Piece, color: Color, square: Square) {
        self.state.pieces[piece].clear(square);
        self.state.colors[color].clear(square);
        self.state.hash ^= ZOBRIST.pieces[color][piece][square];
        if UPDATE_NNUE {
            self.nnue.deactivate(color, piece, square);
        }
    }

    /// Calculates the score of the current position from the perspective of the side to move.
    pub fn evaluate(&self) -> i32 {
        let eval = self.nnue.evaluate(self.side_to_move);
        // Clamp static evaluation within mate bounds
        eval.clamp(-Score::MATE_BOUND + 1, Score::MATE_BOUND - 1)
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
            .any(|state| state.hash == self.state.hash)
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

    /// Returns `true` if the king of the current turn color is in check.
    ///
    /// # Panics
    ///
    /// Panics if there is no king on the board.
    pub fn is_in_check(&self) -> bool {
        let king = self.our(Piece::King).pop();
        self.is_under_attack(king)
    }

    /// Returns `true` if any enemy piece can attack the `Square` of the current turn color.
    pub fn is_under_attack(&self, square: Square) -> bool {
        self.is_square_attacked(square, self.side_to_move)
    }

    /// Returns `true` if any enemy piece can attack the `Square` of the specified `Color`.
    pub fn is_square_attacked(&self, square: Square, color: Color) -> bool {
        let possible_attackers = (self.get_attacks(square, Piece::Knight) & self.pieces(Piece::Knight))
            | (self.get_attacks(square, Piece::Bishop) & (self.pieces(Piece::Bishop) | self.pieces(Piece::Queen)))
            | (self.get_attacks(square, Piece::Rook) & (self.pieces(Piece::Rook) | self.pieces(Piece::Queen)))
            | (self.get_attacks(square, Piece::King) & self.pieces(Piece::King))
            | (crate::lookup::pawn_attacks(square, color) & self.pieces(Piece::Pawn));

        !(possible_attackers & self.colors(!color)).is_empty()
    }

    /// Returns a `Bitboard` with all the squares that the specified `Piece` type can attack.
    ///
    /// # Panics
    ///
    /// Panics if the piece is a pawn because pawn attacks are color dependent.
    pub fn get_attacks(&self, square: Square, piece: Piece) -> Bitboard {
        use crate::lookup::{bishop_attacks, king_attacks, knight_attacks, queen_attacks, rook_attacks};
        match piece {
            Piece::Knight => knight_attacks(square),
            Piece::Bishop => bishop_attacks(square, self.occupancies()),
            Piece::Rook => rook_attacks(square, self.occupancies()),
            Piece::Queen => queen_attacks(square, self.occupancies()),
            Piece::King => king_attacks(square),
            Piece::Pawn => panic!("get_attacks() should not be called for `Piece::Pawn`"),
            Piece::None => panic!("get_attacks() should not be called for `Piece::None`"),
        }
    }

    /// Estimates the resulting Zobrist hash key after making the move.
    pub fn key_after(&self, mv: Move) -> u64 {
        let piece = self.piece_on(mv.start());
        let start = mv.start();
        let target = mv.target();

        let mut key = self.state.hash;

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
}

impl Default for Board {
    fn default() -> Self {
        Self {
            side_to_move: Color::White,
            ply: Default::default(),
            state: InternalState::default(),
            state_stack: Vec::default(),
            move_stack: Vec::default(),
            nnue: Network::default(),
        }
    }
}
