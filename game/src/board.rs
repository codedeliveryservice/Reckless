use crate::core::{bitboard::Bitboard, color::Color, piece::Piece};

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
}
