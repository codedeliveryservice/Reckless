use crate::core::{bitboard::Bitboard, color::Color, moves::Move, piece::Piece, square::Square};

use self::{
    fen::{Fen, ParseFenError},
    generator::Generator,
    state::State,
};

pub mod generator;
pub mod state;

mod fen;

/// Data structure representing the board and the location of its pieces.
pub struct Board {
    pub turn: Color,
    pub state: State,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
}

impl Board {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is invalid.
    pub fn from_fen(fen: &str) -> Result<Self, ParseFenError> {
        Fen::parse(fen)
    }

    /// Generates all possible pseudo legal moves for the current state of `self`.
    pub fn generate_moves(&self) -> Vec<Move> {
        Generator::generate_moves(self)
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    #[inline(always)]
    pub fn us(&self) -> Bitboard {
        self.colors[self.turn as usize]
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    #[inline(always)]
    pub fn them(&self) -> Bitboard {
        self.colors[self.turn.opposite() as usize]
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.them()
    }

    /// Finds a piece on the specified `Square` and returns `Some(Piece)`, if found; otherwise `None`.
    #[inline(always)]
    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        for index in 0..Piece::NUM {
            if self.pieces[index].contains(square) {
                return unsafe { core::mem::transmute(index as u8) };
            }
        }

        None
    }

    /// Places a piece of the specified type and color on the square.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].set(square);
        self.colors[color as usize].set(square);
    }

    /// Removes a piece of the specified type and color from the square.
    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].clear(square);
        self.colors[color as usize].clear(square);
    }

    /// Moves a piece of the specified type and color from the starting square to the target square.
    #[inline(always)]
    pub fn move_piece(&mut self, piece: Piece, color: Color, start: Square, target: Square) {
        self.add_piece(piece, color, target);
        self.remove_piece(piece, color, start);
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            turn: Color::White,
            state: Default::default(),
            pieces: Default::default(),
            colors: Default::default(),
        }
    }
}
