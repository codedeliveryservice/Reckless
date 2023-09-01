use crate::{Bitboard, Castling, Color, Move, MoveList, Piece, Score, Square, Zobrist};

use self::evaluation::Evaluation;

mod evaluation;
mod fen;
mod generator;
mod player;

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Default, Clone, Copy)]
pub(super) struct InternalState {
    hash: Zobrist,
    en_passant: Option<Square>,
    castling: Castling,
    halfmove_clock: u8,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    evaluation: Evaluation,
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Default, Clone)]
pub struct Board {
    pub turn: Color,
    pub ply: usize,
    state: InternalState,
    state_stack: Vec<InternalState>,
    move_stack: Vec<Move>,
}

impl Board {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    pub fn new(fen: &str) -> Self {
        fen::from_fen(fen)
    }

    /// Returns the board corresponding to the starting position.
    pub fn starting_position() -> Self {
        fen::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    /// Generates all possible pseudo legal moves for the current state of `self`.
    #[inline(always)]
    pub fn generate_moves(&self) -> MoveList {
        generator::Generator::new(self).generate()
    }

    /// Returns the `Zobrist` hash key for the current position.
    #[inline(always)]
    pub fn hash(&self) -> Zobrist {
        self.state.hash
    }

    /// Returns a `Bitboard` for the specified `Color`.
    #[inline(always)]
    pub fn colors(&self, color: Color) -> Bitboard {
        self.state.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    #[inline(always)]
    pub fn pieces(&self, piece: Piece) -> Bitboard {
        self.state.pieces[piece]
    }

    /// Returns a `Bitboard` for the specified `Piece` type and `Color`.
    #[inline(always)]
    pub fn of(&self, piece: Piece, color: Color) -> Bitboard {
        self.pieces(piece) & self.colors(color)
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    #[inline(always)]
    pub fn us(&self) -> Bitboard {
        self.colors(self.turn)
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    #[inline(always)]
    pub fn them(&self) -> Bitboard {
        self.colors(self.turn.opposite())
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces(piece) & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces(piece) & self.them()
    }

    /// Finds a piece on the specified `Square` and returns `Some(Piece)`, if found; otherwise `None`.
    #[inline(always)]
    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        for index in 0..Piece::NUM {
            if self.state.pieces[index].contains(square) {
                return Some(Piece::from(index as u8));
            }
        }
        None
    }

    /// Places a piece of the specified type and color on the square.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.state.pieces[piece as usize].set(square);
        self.state.colors[color as usize].set(square);
        self.state.hash.update_piece(piece, color, square);
        self.state.evaluation.add_piece(piece, color, square);
    }

    /// Removes a piece of the specified type and color from the square.
    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.state.pieces[piece as usize].clear(square);
        self.state.colors[color as usize].clear(square);
        self.state.hash.update_piece(piece, color, square);
        self.state.evaluation.remove_piece(piece, color, square);
    }

    /// Returns an incrementally updated scores for both middle game and endgame
    /// phases based on the piece-square tables.
    pub fn psq_score(&self) -> (Score, Score) {
        self.state.evaluation.score()
    }

    /// Returns `true` if the current position has already been present at least once
    /// in the board's history.
    ///
    /// This method does not count the number of encounters.
    #[inline(always)]
    pub fn is_repetition(&self) -> bool {
        self.state_stack.iter().rev().any(|state| state.hash == self.hash())
    }

    /// Returns `true` if the position is a draw by the fifty-move rule.
    pub fn is_fifty_move_draw(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    /// Returns `true` if the last move made was a null move.
    pub fn is_last_move_null(&self) -> bool {
        self.move_stack.last() == Some(&Move::default())
    }

    /// Returns `true` if the king of the current turn color is in check.
    ///
    /// # Panics
    ///
    /// Panics if there is no king on the board.
    #[inline(always)]
    pub fn is_in_check(&mut self) -> bool {
        let king = self.our(Piece::King).pop().unwrap();
        self.is_under_attack(king)
    }

    /// Returns `true` if any enemy piece can attack the `Square`.    
    #[inline(always)]
    pub fn is_under_attack(&self, square: Square) -> bool {
        self.is_square_attacked(square, self.turn.opposite())
    }

    /// Returns `true` if any piece of the attacker `Color` can attack the `Square`.
    pub fn is_square_attacked(&self, square: Square, color: Color) -> bool {
        use crate::lookup;

        let occupancies = self.them() | self.us();

        let bishop_queen = self.pieces(Piece::Bishop) | self.pieces(Piece::Queen);
        let rook_queen = self.pieces(Piece::Rook) | self.pieces(Piece::Queen);

        let possible_attackers = (lookup::king_attacks(square) & self.pieces(Piece::King))
            | (lookup::knight_attacks(square) & self.pieces(Piece::Knight))
            | (lookup::bishop_attacks(square, occupancies) & bishop_queen)
            | (lookup::rook_attacks(square, occupancies) & rook_queen)
            | (lookup::pawn_attacks(square, color.opposite()) & self.pieces(Piece::Pawn));

        (possible_attackers & self.colors(color)).is_not_empty()
    }

    /// Performs Zobrist hashing on `self`, generating an *almost* unique
    /// position hash key from scratch.
    ///
    /// This method should only be used for the initial hash key generation.
    /// For further reference, use `self.hash_key` to get a key that is
    /// incrementally updated during the game due to performance considerations.
    pub fn generate_hash_key(&self) -> Zobrist {
        let mut hash = Zobrist::default();

        for piece in 0..Piece::NUM {
            let piece = Piece::from(piece as u8);

            for square in self.of(piece, Color::White) {
                hash.update_piece(piece, Color::White, square);
            }

            for square in self.of(piece, Color::Black) {
                hash.update_piece(piece, Color::Black, square);
            }
        }

        hash.update_en_passant(self.state.en_passant);
        hash.update_castling(self.state.castling);

        if self.turn == Color::White {
            hash.update_side();
        }

        hash
    }
}
