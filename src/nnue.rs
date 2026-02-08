mod accumulator;
mod threats;

#[cfg(all(
    target_feature = "avx512vl",
    target_feature = "avx512bw",
    target_feature = "gfni",
    target_feature = "avx512vbmi"
))]
mod rays;

pub use threats::initialize;

use crate::{
    board::{Board, BoardObserver},
    nnue::accumulator::{ThreatAccumulator, ThreatDelta},
    types::{Color, MAX_PLY, Move, Piece, PieceType, Score, Square},
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

const NETWORK_SCALE: i32 = 265;

const INPUT_BUCKETS: usize = 10;
const OUTPUT_BUCKETS: usize = 8;

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

const L1_INV_K: f32 = (1.0 / 6.0) as f32;
const L1_OFFSET: f32 = 0.5_f32;
const L2_INV_K: f32 = (1.0 / 6.0) as f32;
const L2_OFFSET: f32 = 0.5_f32;

#[rustfmt::skip]
const INPUT_BUCKETS_LAYOUT: [usize; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,
    4, 5, 6, 7, 7, 6, 5, 4,
    8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
];

#[rustfmt::skip]
const OUTPUT_BUCKETS_LAYOUT: [usize; 33] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1,
    2, 2, 2,
    3, 3, 3,
    4, 4, 4,
    5, 5, 5,
    6, 6, 6,
    7, 7, 7, 7,
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

    #[cfg(not(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    )))]
    pub fn push_threats_on_change(&mut self, board: &Board, piece: Piece, square: Square, add: bool) {
        self.push_threats_single(board, board.occupancies(), piece, square, add);
    }

    #[cfg(not(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    )))]
    pub fn push_threats_on_move(&mut self, board: &Board, piece: Piece, from: Square, to: Square) {
        let occupancies = board.occupancies() ^ to.to_bb();
        self.push_threats_single(board, occupancies, piece, from, false);
        self.push_threats_single(board, occupancies, piece, to, true);
    }

    #[cfg(not(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    )))]
    fn push_threats_single(
        &mut self, board: &Board, occupancies: crate::types::Bitboard, piece: Piece, square: Square, add: bool,
    ) {
        use crate::lookup::{
            attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, ray_pass, rook_attacks,
        };

        let deltas = &mut self.threat_stack[self.index].delta;

        let attacked = attacks(piece, square, occupancies) & occupancies;
        for to in attacked {
            deltas.push(ThreatDelta::new(piece, square, board.piece_on(to), to, add));
        }

        let rook_attacks = rook_attacks(square, occupancies);
        let bishop_attacks = bishop_attacks(square, occupancies);
        let queen_attacks = rook_attacks | bishop_attacks;

        let diagonal = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & bishop_attacks;
        let orthogonal = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & rook_attacks;

        for from in (diagonal | orthogonal) & occupancies {
            let sliding_piece = board.piece_on(from);
            let threatened = ray_pass(from, square) & occupancies & queen_attacks;

            if let Some(to) = threatened.into_iter().next() {
                deltas.push(ThreatDelta::new(sliding_piece, from, board.piece_on(to), to, !add));
            }

            deltas.push(ThreatDelta::new(sliding_piece, from, piece, square, add));
        }

        let black_pawns = board.of(PieceType::Pawn, Color::Black) & pawn_attacks(square, Color::White);
        let white_pawns = board.of(PieceType::Pawn, Color::White) & pawn_attacks(square, Color::Black);

        let knights = board.pieces(PieceType::Knight) & knight_attacks(square);
        let kings = board.pieces(PieceType::King) & king_attacks(square);

        for from in (black_pawns | white_pawns | knights | kings) & occupancies {
            deltas.push(ThreatDelta::new(board.piece_on(from), from, piece, square, add));
        }
    }

    #[cfg(not(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    )))]
    pub fn push_threats_on_mutate(&mut self, board: &Board, old_piece: Piece, new_piece: Piece, square: Square) {
        use crate::lookup::{attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

        let deltas = &mut self.threat_stack[self.index].delta;

        let occupancies = board.occupancies();

        let attacked = attacks(old_piece, square, occupancies) & occupancies;
        for to in attacked {
            deltas.push(ThreatDelta::new(old_piece, square, board.piece_on(to), to, false));
        }
        let attacked = attacks(new_piece, square, occupancies) & occupancies;
        for to in attacked {
            deltas.push(ThreatDelta::new(new_piece, square, board.piece_on(to), to, true));
        }

        let rook_attacks = rook_attacks(square, occupancies);
        let bishop_attacks = bishop_attacks(square, occupancies);

        let diagonal = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & bishop_attacks;
        let orthogonal = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & rook_attacks;

        let black_pawns = board.of(PieceType::Pawn, Color::Black) & pawn_attacks(square, Color::White);
        let white_pawns = board.of(PieceType::Pawn, Color::White) & pawn_attacks(square, Color::Black);

        let knights = board.pieces(PieceType::Knight) & knight_attacks(square);
        let kings = board.pieces(PieceType::King) & king_attacks(square);

        for from in black_pawns | white_pawns | knights | kings | diagonal | orthogonal {
            deltas.push(ThreatDelta::new(board.piece_on(from), from, old_piece, square, false));
            deltas.push(ThreatDelta::new(board.piece_on(from), from, new_piece, square, true));
        }
    }

    #[cfg(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    ))]
    pub fn push_threats_on_change(&mut self, board: &Board, piece: Piece, square: Square, add: bool) {
        use rays::*;
        use std::arch::x86_64::*;

        let deltas = &mut self.threat_stack[self.index].delta;

        let (perm, valid) = ray_permutation(square);
        let (pboard, rays) = board_to_rays(perm, valid, unsafe { board.mailbox_vector() });
        let occupied = unsafe { _mm512_test_epi8_mask(rays, rays) };

        let closest = closest_on_rays(occupied);
        let attacked = attacking_along_rays(piece, closest);
        let attackers = attackers_along_rays(rays) & closest;
        let sliders = sliders_along_rays(rays) & closest;

        Self::splat_threats(deltas, true, pboard, perm, attacked, piece, square, add);
        Self::splat_threats(deltas, false, pboard, perm, attackers, piece, square, add);

        // Deal with x-rays
        unsafe {
            let nadd = (!add as u32) << 31;
            let nadd = _mm_set1_epi32(nadd as i32);

            let victim_mask = (closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
            let xray_valid = ray_fill(victim_mask) & ray_fill(sliders);

            unsafe fn compress(m: u64, v: __m512i) -> __m128i {
                _mm512_castsi512_si128(_mm512_maskz_compress_epi8(m, v))
            }

            let p1 = compress(sliders & xray_valid, pboard);
            let sq1 = compress(sliders & xray_valid, perm);
            let p2 = compress(victim_mask & xray_valid, rays::flip_rays(pboard));
            let sq2 = compress(victim_mask & xray_valid, rays::flip_rays(perm));

            let pair1 = _mm_unpacklo_epi8(p1, sq1);
            let pair2 = _mm_unpacklo_epi8(p2, sq2);

            deltas.unchecked_write(|data| {
                _mm_storeu_si128(data.cast(), _mm_or_si128(_mm_unpacklo_epi16(pair1, pair2), nadd));
                _mm_storeu_si128(data.add(4).cast(), _mm_or_si128(_mm_unpackhi_epi16(pair1, pair2), nadd));
                (sliders & xray_valid).count_ones() as usize
            });
        }
    }

    #[cfg(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    ))]
    pub fn push_threats_on_move(&mut self, board: &Board, piece: Piece, src: Square, dst: Square) {
        use rays::*;
        use std::arch::x86_64::*;

        let deltas = &mut self.threat_stack[self.index].delta;

        let board = unsafe {
            _mm512_mask_blend_epi8(dst.to_bb().0, board.mailbox_vector(), _mm512_set1_epi8(Piece::None as i8))
        };

        let (src_perm, src_valid) = ray_permutation(src);
        let (dst_perm, dst_valid) = ray_permutation(dst);
        let (src_pboard, src_rays) = board_to_rays(src_perm, src_valid, board);
        let (dst_pboard, dst_rays) = board_to_rays(dst_perm, dst_valid, board);
        let src_occupied = unsafe { _mm512_test_epi8_mask(src_rays, src_rays) };
        let dst_occupied = unsafe { _mm512_test_epi8_mask(dst_rays, dst_rays) };

        let src_closest = closest_on_rays(src_occupied);
        let dst_closest = closest_on_rays(dst_occupied);
        let src_attacked = attacking_along_rays(piece, src_closest);
        let dst_attacked = attacking_along_rays(piece, dst_closest);
        let src_attackers = attackers_along_rays(src_rays) & src_closest;
        let dst_attackers = attackers_along_rays(dst_rays) & dst_closest;
        let src_sliders = sliders_along_rays(src_rays) & src_closest;
        let dst_sliders = sliders_along_rays(dst_rays) & dst_closest;

        Self::splat_threats(deltas, true, src_pboard, src_perm, src_attacked, piece, src, false);
        Self::splat_threats(deltas, false, src_pboard, src_perm, src_attackers, piece, src, false);
        Self::splat_threats(deltas, true, dst_pboard, dst_perm, dst_attacked, piece, dst, true);
        Self::splat_threats(deltas, false, dst_pboard, dst_perm, dst_attackers, piece, dst, true);

        // Deal with x-rays
        unsafe {
            let src_victim = (src_closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
            let dst_victim = (dst_closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
            let src_xray_valid = ray_fill(src_victim) & ray_fill(src_sliders);
            let dst_xray_valid = ray_fill(dst_victim) & ray_fill(dst_sliders);

            unsafe fn compress(m: u64, v: __m512i) -> __m128i {
                _mm512_castsi512_si128(_mm512_maskz_compress_epi8(m, v))
            }

            let src_p1 = compress(src_sliders & src_xray_valid, src_pboard);
            let dst_p1 = compress(dst_sliders & dst_xray_valid, dst_pboard);
            let src_sq1 = compress(src_sliders & src_xray_valid, src_perm);
            let dst_sq1 = compress(dst_sliders & dst_xray_valid, dst_perm);
            let src_p2 = compress(src_victim & src_xray_valid, flip_rays(src_pboard));
            let dst_p2 = compress(dst_victim & dst_xray_valid, flip_rays(dst_pboard));
            let src_sq2 = compress(src_victim & src_xray_valid, flip_rays(src_perm));
            let dst_sq2 = compress(dst_victim & dst_xray_valid, flip_rays(dst_perm));

            let src_pair1 = _mm_unpacklo_epi8(src_p1, src_sq1);
            let dst_pair1 = _mm_unpacklo_epi8(dst_p1, dst_sq1);
            let src_pair2 = _mm_unpacklo_epi8(src_p2, src_sq2);
            let dst_pair2 = _mm_unpacklo_epi8(dst_p2, dst_sq2);

            deltas.unchecked_write(|data| {
                let add = _mm_set1_epi32(0x80000000u32 as i32);
                _mm_storeu_si128(data.cast(), _mm_or_si128(_mm_unpacklo_epi16(src_pair1, src_pair2), add));
                _mm_storeu_si128(data.add(4).cast(), _mm_or_si128(_mm_unpackhi_epi16(src_pair1, src_pair2), add));
                (src_sliders & src_xray_valid).count_ones() as usize
            });
            deltas.unchecked_write(|data| {
                _mm_storeu_si128(data.cast(), _mm_unpacklo_epi16(dst_pair1, dst_pair2));
                _mm_storeu_si128(data.add(4).cast(), _mm_unpackhi_epi16(dst_pair1, dst_pair2));
                (dst_sliders & dst_xray_valid).count_ones() as usize
            });
        }
    }

    #[cfg(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    ))]
    pub fn push_threats_on_mutate(&mut self, board: &Board, old_piece: Piece, new_piece: Piece, square: Square) {
        use rays::*;
        use std::arch::x86_64::*;

        let deltas = &mut self.threat_stack[self.index].delta;

        let (perm, valid) = ray_permutation(square);
        let (pboard, rays) = board_to_rays(perm, valid, unsafe { board.mailbox_vector() });
        let occupied = unsafe { _mm512_test_epi8_mask(rays, rays) };

        let closest = closest_on_rays(occupied);
        let old_attacked = attacking_along_rays(old_piece, closest);
        let new_attacked = attacking_along_rays(new_piece, closest);
        let attackers = attackers_along_rays(rays) & closest;

        Self::splat_threats(deltas, true, pboard, perm, old_attacked, old_piece, square, false);
        Self::splat_threats(deltas, false, pboard, perm, attackers, old_piece, square, false);
        Self::splat_threats(deltas, true, pboard, perm, new_attacked, new_piece, square, true);
        Self::splat_threats(deltas, false, pboard, perm, attackers, new_piece, square, true);
    }

    #[inline]
    #[cfg(all(
        target_feature = "avx512vl",
        target_feature = "avx512bw",
        target_feature = "gfni",
        target_feature = "avx512vbmi"
    ))]
    #[allow(clippy::too_many_arguments)]
    fn splat_threats(
        deltas: &mut crate::types::ArrayVec<ThreatDelta, 80>, is_to: bool, pboard: std::arch::x86_64::__m512i,
        perm: std::arch::x86_64::__m512i, bitray: u64, p2: Piece, sq2: Square, add: bool,
    ) {
        use std::arch::x86_64::*;

        unsafe {
            let add = (add as u32) << 31;
            let add = _mm512_set1_epi32(add as i32);

            let template = {
                let pair = p2 as u16 | ((sq2 as u16) << 8);
                _mm512_set1_epi16(pair as i16)
            };

            let iota = _mm512_maskz_compress_epi8(bitray, perm);
            let mailbox = _mm512_maskz_compress_epi8(bitray, pboard);

            let idx = _mm512_set_epi8(
                79, 15, 79, 15, 78, 14, 78, 14, 77, 13, 77, 13, 76, 12, 76, 12, 75, 11, 75, 11, 74, 10, 74, 10, 73, 9,
                73, 9, 72, 8, 72, 8, 71, 7, 71, 7, 70, 6, 70, 6, 69, 5, 69, 5, 68, 4, 68, 4, 67, 3, 67, 3, 66, 2, 66,
                2, 65, 1, 65, 1, 64, 0, 64, 0,
            );

            let widen = _mm512_permutex2var_epi8(mailbox, idx, iota);
            let mask = if is_to { 0xCCCCCCCCCCCCCCCC } else { 0x3333333333333333 };

            let vector = _mm512_or_si512(_mm512_mask_mov_epi8(template, mask, widen), add);

            deltas.unchecked_write(|data| {
                _mm512_storeu_si512(data.cast(), vector);
                bitray.count_ones() as usize
            });
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
                && ((from.file() >= 4) != (to.file() >= 4) || INPUT_BUCKETS_LAYOUT[from] != INPUT_BUCKETS_LAYOUT[to])
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
        let bucket = OUTPUT_BUCKETS_LAYOUT[board.occupancies().popcount()];

        unsafe {
            let ft_out =
                forward::activate_ft(&self.pst_stack[self.index], &self.threat_stack[self.index], board.side_to_move());
            let (nnz_indexes, nnz_count) = forward::find_nnz(&ft_out, &self.nnz_table);

            let l1_out = forward::propagate_l1(ft_out, &nnz_indexes[..nnz_count], bucket);
            let l2_out = forward::propagate_l2(l1_out, bucket);
            let l3_out = forward::propagate_l3(l2_out, bucket);

            ((l3_out * NETWORK_SCALE as f32) as i32).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
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

impl BoardObserver for Network {
    fn on_piece_move(&mut self, board: &Board, piece: Piece, from: Square, to: Square) {
        self.push_threats_on_move(board, piece, from, to);
    }

    fn on_piece_mutate(&mut self, board: &Board, old_piece: Piece, new_piece: Piece, square: Square) {
        self.push_threats_on_mutate(board, old_piece, new_piece, square);
    }

    fn on_piece_change(&mut self, board: &Board, piece: Piece, square: Square, add: bool) {
        self.push_threats_on_change(board, piece, square, add);
    }
}

#[repr(C)]
struct Parameters {
    ft_threat_weights: Aligned<[[i8; L1_SIZE]; 66864]>,
    ft_piece_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * 768]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[[i8; L2_SIZE * L1_SIZE]; OUTPUT_BUCKETS]>,
    l1_biases: Aligned<[[f32; L2_SIZE]; OUTPUT_BUCKETS]>,
    l2_weights: Aligned<[[[f32; L3_SIZE]; L2_SIZE]; OUTPUT_BUCKETS]>,
    l2_biases: Aligned<[[f32; L3_SIZE]; OUTPUT_BUCKETS]>,
    l3_weights: Aligned<[[f32; L3_SIZE]; OUTPUT_BUCKETS]>,
    l3_biases: Aligned<[f32; OUTPUT_BUCKETS]>,
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
