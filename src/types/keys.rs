use crate::types::{Castling, Color, Piece, Square, ZOBRIST};

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

    #[cfg(not(target_feature = "avx2"))]
    fn update_full(&mut self, key: u64) {
        self.full ^= key;
    }

    #[cfg(not(target_feature = "avx2"))]
    fn update_piece(&mut self, piece: Piece, piece_key: u64) {
        use crate::types::PieceType;

        self.full ^= piece_key;

        match piece.piece_type() {
            PieceType::Pawn => self.pawn ^= piece_key,
            _ => self.non_pawn[piece.color()] ^= piece_key,
        }
    }

    #[cfg(target_feature = "avx2")]
    fn update_full(&mut self, key: u64) {
        use std::arch::x86_64::*;

        unsafe {
            let ptr = self as *mut Keys as *mut __m256i;
            let keys = _mm256_loadu_si256(ptr);
            let key = _mm256_zextsi128_si256(_mm_cvtsi64_si128(key as i64));
            _mm256_storeu_si256(ptr, _mm256_xor_si256(keys, key));
        }
    }

    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    fn update_piece(&mut self, piece: Piece, piece_key: u64) {
        use std::arch::x86_64::*;

        unsafe {
            let ptr = self as *mut Keys as *mut __m256i;
            let keys = _mm256_loadu_si256(ptr);
            let piece_key = _mm256_and_si256(_mm256_set1_epi64x(piece_key as i64), PIECE_MASK[piece]);
            _mm256_storeu_si256(ptr, _mm256_xor_si256(keys, piece_key));
        }
    }

    #[cfg(target_feature = "avx512f")]
    fn update_piece(&mut self, piece: Piece, piece_key: u64) {
        use std::arch::x86_64::*;

        unsafe {
            let ptr = self as *mut Keys as *mut __m256i;
            let keys = _mm256_loadu_si256(ptr);
            let piece_key = _mm256_maskz_set1_epi64(PIECE_MASK[piece], piece_key as i64);
            _mm256_storeu_si256(ptr, _mm256_xor_si256(keys, piece_key));
        }
    }
}

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
type PieceMask = std::arch::x86_64::__m256i;
#[cfg(target_feature = "avx512f")]
type PieceMask = u8;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
const PAWN: PieceMask = unsafe { std::mem::transmute::<[i64; 4], PieceMask>([-1, -1, 0, 0]) };
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
const WHITE_NON_PAWN: PieceMask = unsafe { std::mem::transmute::<[i64; 4], PieceMask>([-1, 0, -1, 0]) };
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
const BLACK_NON_PAWN: PieceMask = unsafe { std::mem::transmute::<[i64; 4], PieceMask>([-1, 0, 0, -1]) };

#[cfg(target_feature = "avx512f")]
const PAWN: PieceMask = 0b0011;
#[cfg(target_feature = "avx512f")]
const WHITE_NON_PAWN: PieceMask = 0b0101;
#[cfg(target_feature = "avx512f")]
const BLACK_NON_PAWN: PieceMask = 0b1001;

#[cfg(all(target_feature = "avx2"))]
const PIECE_MASK: [PieceMask; 12] = [
    PAWN,
    PAWN,
    WHITE_NON_PAWN,
    BLACK_NON_PAWN,
    WHITE_NON_PAWN,
    BLACK_NON_PAWN,
    WHITE_NON_PAWN,
    BLACK_NON_PAWN,
    WHITE_NON_PAWN,
    BLACK_NON_PAWN,
    WHITE_NON_PAWN,
    BLACK_NON_PAWN,
];
