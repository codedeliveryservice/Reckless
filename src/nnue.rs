mod accumulator;
mod threats;

pub use threats::initialize;

use crate::{
    board::Board,
    lookup::{attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, ray_pass, rook_attacks},
    nnue::accumulator::{ThreatAccumulator, ThreatDelta},
    types::{ArrayVec, Bitboard, Color, Move, Piece, PieceType, Square, MAX_PLY},
};

use accumulator::{AccumulatorCache, PstAccumulator};

mod forward {
    #[cfg(any(target_feature = "avx2", target_feature = "neon"))]
    mod vectorized;
    #[cfg(any(target_feature = "avx2", target_feature = "neon"))]
    pub use vectorized::*;

    #[cfg(not(any(target_feature = "avx2", target_feature = "neon")))]
    mod scalar;
    #[cfg(not(any(target_feature = "avx2", target_feature = "neon")))]
    pub use scalar::*;
}

mod simd {
    #[cfg(target_feature = "avx512f")]
    mod avx512;
    #[cfg(target_feature = "avx512f")]
    pub use avx512::*;

    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    mod avx2;
    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    pub use avx2::*;

    #[cfg(all(target_feature = "neon", not(any(target_feature = "avx2", target_feature = "avx512f"))))]
    mod neon;
    #[cfg(all(target_feature = "neon", not(any(target_feature = "avx2", target_feature = "avx512f"))))]
    pub use neon::*;

    #[cfg(not(any(target_feature = "avx512f", target_feature = "avx2", target_feature = "neon")))]
    mod scalar;
    #[cfg(not(any(target_feature = "avx512f", target_feature = "avx2", target_feature = "neon")))]
    pub use scalar::*;
}

const NETWORK_SCALE: i32 = 380;

const INPUT_BUCKETS: usize = 10;

const L1_SIZE: usize = 512;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;

const FT_QUANT: i32 = 255;
const L1_QUANT: i32 = 64;

#[cfg(target_feature = "avx512f")]
const FT_SHIFT: u32 = 9;
#[cfg(not(target_feature = "avx512f"))]
const FT_SHIFT: i32 = 9;

const DEQUANT_MULTIPLIER: f32 = (1 << FT_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

#[rustfmt::skip]
const BUCKETS: [usize; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,
    4, 5, 6, 7, 7, 6, 5, 4,
    8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
];

#[repr(align(16))]
#[derive(Clone, Copy)]
struct SparseEntry {
    indexes: [u16; 8],
    count: usize,
}

#[derive(Clone)]
pub struct Network {
    index: usize,
    pst_stack: Box<[PstAccumulator]>,
    threat_stack: Box<[ThreatAccumulator]>,
    cache: AccumulatorCache,
    nnz_table: Box<[SparseEntry]>,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv.is_some());

        self.index += 1;

        self.pst_stack[self.index].accurate = [false; 2];
        self.pst_stack[self.index].delta.mv = mv;
        self.pst_stack[self.index].delta.piece = board.piece_on(mv.from());
        self.pst_stack[self.index].delta.captured = board.piece_on(mv.to());

        self.threat_stack[self.index].accurate = [false; 2];
        self.threat_stack[self.index].delta.clear();
    }

    pub fn push_threats(&mut self, board: &Board, piece: Piece, square: Square, add: bool) {
        let deltas = &mut self.threat_stack[self.index].delta;

        let attacked = attacks(piece, square, board.occupancies()) & board.occupancies();
        Self::splat_threats(deltas, true, board, attacked, piece, square, add);

        let rook_attacks = rook_attacks(square, board.occupancies());
        let bishop_attacks = bishop_attacks(square, board.occupancies());
        let queen_attacks = rook_attacks | bishop_attacks;

        let diagonal = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & bishop_attacks;
        let orthogonal = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & rook_attacks;

        for from in diagonal | orthogonal {
            let sliding_piece = board.piece_on(from);
            let threatened = ray_pass(from, square) & board.occupancies() & queen_attacks;

            if let Some(to) = threatened.into_iter().next() {
                deltas.push(ThreatDelta::new(sliding_piece, from, board.piece_on(to), to, !add));
            }

            deltas.push(ThreatDelta::new(sliding_piece, from, piece, square, add));
        }

        let black_pawns = board.of(PieceType::Pawn, Color::Black) & pawn_attacks(square, Color::White);
        let white_pawns = board.of(PieceType::Pawn, Color::White) & pawn_attacks(square, Color::Black);

        let knights = board.pieces(PieceType::Knight) & knight_attacks(square);
        let kings = board.pieces(PieceType::King) & king_attacks(square);

        let attackers = black_pawns | white_pawns | knights | kings;
        Self::splat_threats(deltas, false, board, attackers, piece, square, add);
    }

    #[inline]
    #[cfg(not(all(target_feature = "avx512vl", target_feature = "avx512bw", target_feature = "avx512vbmi")))]
    fn splat_threats(
        deltas: &mut ArrayVec<ThreatDelta, 80>, is_to: bool, board: &Board, bb: Bitboard, p2: Piece, sq2: Square,
        add: bool,
    ) {
        if is_to {
            for sq1 in bb {
                let p1 = board.piece_on(sq1);
                deltas.push(ThreatDelta::new(p2, sq2, p1, sq1, add));
            }
        } else {
            for sq1 in bb {
                let p1 = board.piece_on(sq1);
                deltas.push(ThreatDelta::new(p1, sq1, p2, sq2, add));
            }
        }
    }

    #[inline]
    #[cfg(all(target_feature = "avx512vl", target_feature = "avx512bw", target_feature = "avx512vbmi"))]
    fn splat_threats(
        deltas: &mut ArrayVec<ThreatDelta, 80>, is_to: bool, board: &Board, bb: Bitboard, p2: Piece, sq2: Square,
        add: bool,
    ) {
        use std::arch::x86_64::*;

        unsafe {
            let add = (add as u32) << 31;
            let add = _mm512_set1_epi32(add as i32);

            let template = {
                let pair = p2 as u16 | ((sq2 as u16) << 8);
                _mm512_set1_epi16(pair as i16)
            };

            let iota = _mm512_set_epi8(
                63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39, 38,
                37, 36, 35, 34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12,
                11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
            );
            let mailbox = board.mailbox_vector();

            let iota = _mm512_maskz_compress_epi8(bb.0, iota);
            let mailbox = _mm512_maskz_compress_epi8(bb.0, mailbox);

            let idx = _mm512_set_epi8(
                79, 15, 79, 15, 78, 14, 78, 14, 77, 13, 77, 13, 76, 12, 76, 12, 75, 11, 75, 11, 74, 10, 74, 10, 73, 9,
                73, 9, 72, 8, 72, 8, 71, 7, 71, 7, 70, 6, 70, 6, 69, 5, 69, 5, 68, 4, 68, 4, 67, 3, 67, 3, 66, 2, 66,
                2, 65, 1, 65, 1, 64, 0, 64, 0,
            );

            let widen = _mm512_permutex2var_epi8(mailbox, idx, iota);
            let mask = if is_to { 0xCCCCCCCC } else { 0x33333333 };

            let vector = _mm512_or_si512(_mm512_mask_mov_epi8(template, mask, widen), add);

            deltas.push_vector_unchecked(bb.popcount(), vector);
        }
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn full_refresh(&mut self, board: &Board) {
        self.pst_stack[self.index].refresh(board, Color::White, &mut self.cache);
        self.pst_stack[self.index].refresh(board, Color::Black, &mut self.cache);

        self.threat_stack[self.index].refresh(board, Color::White);
        self.threat_stack[self.index].refresh(board, Color::Black);
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        debug_assert!(self.pst_stack[0].accurate == [true; 2]);
        debug_assert!(self.threat_stack[0].accurate == [true; 2]);

        for pov in [Color::White, Color::Black] {
            if self.pst_stack[self.index].accurate[pov] && self.threat_stack[self.index].accurate[pov] {
                continue;
            }

            match self.can_update_pst(pov) {
                Some(index) => self.update_pst_accumulator(index, board, pov),
                None => self.pst_stack[self.index].refresh(board, pov, &mut self.cache),
            }

            match self.can_update_threats(pov) {
                Some(index) => self.update_threat_accumulator(index, board, pov),
                None => self.threat_stack[self.index].refresh(board, pov),
            }
        }

        self.output_transformer(board)
    }

    fn update_pst_accumulator(&mut self, accurate: usize, board: &Board, pov: Color) {
        let king = board.king_square(pov);

        for i in accurate..self.index {
            if let (prev, [current, ..]) = self.pst_stack.split_at_mut(i + 1) {
                current.update(&prev[i], board, king, pov);
            }
        }
    }

    fn update_threat_accumulator(&mut self, accurate: usize, board: &Board, pov: Color) {
        let king = board.king_square(pov);

        for i in accurate..self.index {
            if let (prev, [current, ..]) = self.threat_stack.split_at_mut(i + 1) {
                unsafe { current.update(&prev[i], king, pov) };
            }
        }
    }

    fn can_update_pst(&self, pov: Color) -> Option<usize> {
        for i in (0..=self.index).rev() {
            if self.pst_stack[i].accurate[pov] {
                return Some(i);
            }

            let delta = &self.pst_stack[i].delta;

            let from = delta.mv.from() ^ (56 * (delta.piece.piece_color() as u8));
            let to = delta.mv.to() ^ (56 * (delta.piece.piece_color() as u8));

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && ((from.file() >= 4) != (to.file() >= 4) || BUCKETS[from] != BUCKETS[to])
            {
                return None;
            }
        }

        None
    }

    fn can_update_threats(&self, pov: Color) -> Option<usize> {
        for i in (0..=self.index).rev() {
            if self.threat_stack[i].accurate[pov] {
                return Some(i);
            }

            let delta = &self.pst_stack[i].delta;

            let from = delta.mv.from();
            let to = delta.mv.to();

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && (from.file() >= 4) != (to.file() >= 4)
            {
                return None;
            }
        }

        None
    }

    fn output_transformer(&self, board: &Board) -> i32 {
        unsafe {
            let ft_out =
                forward::activate_ft(&self.pst_stack[self.index], &self.threat_stack[self.index], board.side_to_move());
            let (nnz_indexes, nnz_count) = forward::find_nnz(&ft_out, &self.nnz_table);

            let l1_out = forward::propagate_l1(ft_out, &nnz_indexes[..nnz_count]);
            let l2_out = forward::propagate_l2(l1_out);
            let l3_out = forward::propagate_l3(l2_out);

            (l3_out * NETWORK_SCALE as f32) as i32
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        let mut nnz_table = vec![SparseEntry { indexes: [0; 8], count: 0 }; 256];

        for (byte, entry) in nnz_table.iter_mut().enumerate() {
            let mut count = 0;

            for bit in 0..8 {
                if (byte & (1 << bit)) != 0 {
                    entry.indexes[count] = bit as u16;
                    count += 1;
                }
            }

            entry.count = count;
        }

        Self {
            index: 0,
            pst_stack: vec![PstAccumulator::new(); MAX_PLY].into_boxed_slice(),
            threat_stack: vec![ThreatAccumulator::new(); MAX_PLY].into_boxed_slice(),
            cache: AccumulatorCache::default(),
            nnz_table: nnz_table.into_boxed_slice(),
        }
    }
}

#[repr(C)]
struct Parameters {
    ft_threat_weights: Aligned<[[i8; L1_SIZE]; 79856]>,
    ft_piece_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * 768]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[i8; L2_SIZE * L1_SIZE]>,
    l1_biases: Aligned<[f32; L2_SIZE]>,
    l2_weights: Aligned<[[f32; L3_SIZE]; L2_SIZE]>,
    l2_biases: Aligned<[f32; L3_SIZE]>,
    l3_weights: Aligned<[f32; L3_SIZE]>,
    l3_biases: f32,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

#[repr(align(64))]
#[derive(Clone)]
struct Aligned<T> {
    data: T,
}

impl<T> Aligned<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> std::ops::Deref for Aligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
