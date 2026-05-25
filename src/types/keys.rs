use crate::types::{Castling, Color, Piece, PieceType, Square, ZOBRIST};

#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub full: u64,
    pub pawn: u64,
    pub non_pawn: [u64; Color::NUM],
}

impl Keys {
    pub fn full(&self) -> u64 {
        self.full
    }

    pub const fn pawn(&self) -> u64 {
        self.pawn
    }

    pub const fn non_pawn(&self, color: Color) -> u64 {
        self.non_pawn[color as usize]
    }

    pub fn toggle(&mut self, piece: Piece, sq: Square) {
        let piece_key = ZOBRIST.pieces[piece][sq];

        self.full ^= piece_key;

        match piece.piece_type() {
            PieceType::Pawn => self.pawn ^= piece_key,
            _ => self.non_pawn[piece.color()] ^= piece_key,
        }
    }

    pub fn toggle_side(&mut self) {
        self.full ^= ZOBRIST.side;
    }

    pub fn toggle_castling(&mut self, castling: Castling) {
        self.full ^= ZOBRIST.castling[castling];
    }

    pub fn toggle_en_passant(&mut self, en_passant: Square) {
        self.full ^= ZOBRIST.en_passant[en_passant];
    }
}
