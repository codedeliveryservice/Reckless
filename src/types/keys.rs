use crate::types::{Color, Piece, PieceType, Square, ZOBRIST};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub full: u64,
    pub pawn: u64,
    pub non_pawn: [u64; 2],
}

impl Keys {
    pub const fn zero() -> Self {
        Self { full: 0, pawn: 0, non_pawn: [0; 2] }
    }

    pub fn update_full(&mut self, value: u64) {
        self.full ^= value;
    }

    pub fn toggle(&mut self, piece: Piece, sq: Square) {
        let piece_key = ZOBRIST.pieces[piece as usize][sq as usize];

        self.full ^= piece_key;

        match piece.piece_type() {
            PieceType::Pawn => self.pawn ^= piece_key,
            _ => self.non_pawn[piece.color() as usize] ^= piece_key,
        }
    }

    pub fn full(&self) -> u64 {
        self.full
    }

    pub const fn pawn(&self) -> u64 {
        self.pawn
    }

    pub const fn non_pawn(&self, color: Color) -> u64 {
        self.non_pawn[color as usize]
    }
}
