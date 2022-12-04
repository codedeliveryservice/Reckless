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

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].set(square);
        self.colors[color as usize].set(square);
    }
}
