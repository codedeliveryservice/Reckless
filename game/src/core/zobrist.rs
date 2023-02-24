//! Provides function for Zobrist hashing.
//!
//! See (Zobrist Hashing)[https://www.chessprogramming.org/Zobrist_Hashing]
//! for more information.
use crate::{Board, Castling, Color, Piece, Square};

include!(concat!(env!("OUT_DIR"), "/zobrist.rs"));

/// Represents an *almost* unique hash key encoded as a 64-bit unsigned integer.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Zobrist(pub u64);

impl Zobrist {
    /// Generates a new `Zobrist` hash key from scratch for the `Board`.
    pub(crate) fn new(board: &Board) -> Self {
        let mut hash = Zobrist::default();

        for piece in 0..Piece::NUM {
            let piece = Piece::from(piece as u8);

            for square in board.of(piece, Color::White) {
                hash.update_piece(piece, Color::White, square);
            }

            for square in board.of(piece, Color::Black) {
                hash.update_piece(piece, Color::Black, square);
            }
        }

        hash.update_en_passant(board.state().en_passant);
        hash.update_castling(board.state().castling);

        if board.turn == Color::White {
            hash.update_side();
        }

        hash
    }

    #[inline(always)]
    pub(crate) fn update_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.0 ^= PIECE_KEYS[color][piece][square.0 as usize];
    }

    #[inline(always)]
    pub(crate) fn update_side(&mut self) {
        self.0 ^= SIDE_KEY
    }

    #[inline(always)]
    pub(crate) fn update_castling(&mut self, castling: Castling) {
        self.0 ^= CASTLING_KEYS[castling.0 as usize]
    }

    #[inline(always)]
    pub(crate) fn update_en_passant(&mut self, square: Option<Square>) {
        if let Some(square) = square {
            self.0 ^= EN_PASSANT_KEYS[square.0 as usize]
        }
    }

    #[inline(always)]
    pub(crate) fn update_en_passant_square(&mut self, square: Square) {
        self.0 ^= EN_PASSANT_KEYS[square.0 as usize]
    }
}
