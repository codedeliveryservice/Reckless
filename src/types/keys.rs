use crate::types::{Castling, Color, Piece, PieceType, Square, ZOBRIST};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Keys {
    pub full: u64,
    pub pawn: u64,
    pub non_pawn: [u64; 2],
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
        self.update_piece(piece, ZOBRIST.pieces[piece][sq]);
    }

    pub fn toggle_side(&mut self) {
        self.update_full(ZOBRIST.side);
    }

    pub fn toggle_castling(&mut self, castling: Castling) {
        self.update_full(ZOBRIST.castling[castling]);
    }

    pub fn toggle_en_passant(&mut self, en_passant: Square) {
        self.update_full(ZOBRIST.en_passant[en_passant]);
    }

    fn update_full(&mut self, key: u64) {
        self.full ^= key;
    }

    fn update_piece(&mut self, piece: Piece, piece_key: u64) {
        self.full ^= piece_key;

        match piece.piece_type() {
            PieceType::Pawn => self.pawn ^= piece_key,
            _ => self.non_pawn[piece.color()] ^= piece_key,
        }
    }
}
