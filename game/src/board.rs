use crate::core::{bitboard::Bitboard, color::Color, piece::Piece, square::Square};

use self::{
    fen::{Fen, ParseFenError},
    state::State,
};

pub mod generator;
pub mod state;

mod fen;

#[derive(Default)]
pub struct Board {
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    state: State,
}

impl Board {
    pub fn from_fen(fen: &str) -> Result<Self, ParseFenError> {
        Fen::parse(fen)
    }

    #[inline(always)]
    pub fn us(&self) -> Bitboard {
        self.colors[self.state.turn as usize]
    }

    #[inline(always)]
    pub fn them(&self) -> Bitboard {
        self.colors[self.state.turn.opposite() as usize]
    }

    #[inline(always)]
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.us()
    }

    #[inline(always)]
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.them()
    }

    /// Places a piece of the specified type and color on the square.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].set(square);
        self.colors[color as usize].set(square);
    }
}
