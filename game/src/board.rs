use crate::core::{Bitboard, Color, MoveList, Piece, Square, Zobrist};

use self::{fen::ParseFenError, history::History, repetitions::Repetitions, state::State};

mod fen;
mod generator;
mod history;
mod player;
mod repetitions;
mod state;

/// Data structure representing the board and the location of its pieces.
#[derive(Default, Clone)]
pub struct Board {
    pub turn: Color,
    pub hash_key: Zobrist,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    repetitions: Repetitions,
    history: History,
    state: State,
}

impl Board {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is invalid.
    pub fn new(fen: &str) -> Result<Self, ParseFenError> {
        fen::Fen::parse(fen)
    }

    /// Generates all possible pseudo legal moves for the current state of `self`.
    #[inline(always)]
    pub fn generate_moves(&self) -> MoveList {
        generator::Generator::generate_moves(self)
    }

    /// Returns a `Bitboard` for the specified `Piece` type and `Color`.
    #[inline(always)]
    pub fn of(&self, piece: Piece, color: Color) -> Bitboard {
        self.pieces[piece] & self.colors[color]
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    #[inline(always)]
    pub fn us(&self) -> Bitboard {
        self.colors[self.turn]
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    #[inline(always)]
    pub fn them(&self) -> Bitboard {
        self.colors[self.turn.opposite()]
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces[piece] & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces[piece] & self.them()
    }

    /// Finds a piece on the specified `Square` and returns `Some(Piece)`, if found; otherwise `None`.
    #[inline(always)]
    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        for index in 0..Piece::NUM {
            if self.pieces[index].contains(square) {
                return Some(Piece::from(index as u8));
            }
        }
        None
    }

    /// Places a piece of the specified type and color on the square.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].set(square);
        self.colors[color as usize].set(square);
        self.hash_key.update_piece(piece, color, square);
    }

    /// Removes a piece of the specified type and color from the square.
    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].clear(square);
        self.colors[color as usize].clear(square);
        self.hash_key.update_piece(piece, color, square);
    }

    /// Moves a piece of the specified type and color from the starting square to the target square.
    #[inline(always)]
    pub fn move_piece(&mut self, piece: Piece, color: Color, start: Square, target: Square) {
        self.add_piece(piece, color, target);
        self.remove_piece(piece, color, start);
    }

    /// Returns `true` if the current position has already been present at least once
    /// in the board's history.
    ///
    /// This method does not count the number of encounters.
    #[inline(always)]
    pub fn is_repetition(&self) -> bool {
        self.repetitions.is_repetition(self.hash_key)
    }

    /// Returns `true` if the king of the current turn color is in check.
    ///
    /// # Panics
    ///
    /// Panics if there is no king on the board.
    #[inline(always)]
    pub fn is_in_check(&self) -> bool {
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

        let bishop_queen = self.pieces[Piece::Bishop] | self.pieces[Piece::Queen];
        let rook_queen = self.pieces[Piece::Rook] | self.pieces[Piece::Queen];

        let possible_attackers = (lookup::king_attacks(square) & self.pieces[Piece::King])
            | (lookup::knight_attacks(square) & self.pieces[Piece::Knight])
            | (lookup::bishop_attacks(square, occupancies) & bishop_queen)
            | (lookup::rook_attacks(square, occupancies) & rook_queen)
            | (lookup::pawn_attacks(square, color.opposite()) & self.pieces[Piece::Pawn]);

        (possible_attackers & self.colors[color]).is_not_empty()
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
