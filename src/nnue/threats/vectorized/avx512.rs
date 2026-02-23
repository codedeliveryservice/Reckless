use std::arch::x86_64::*;

use crate::{
    nnue::{
        accumulator::ThreatAccumulator,
        threats::vectorized::{
            PIECE_TO_BIT_TABLE, RAY_ATTACKERS_MASK, RAY_ATTACKS_MASK, RAY_PERMUTATIONS, RAY_SLIDERS_MASK,
        },
    },
    types::{Piece, Square},
};

pub fn ray_permutation(focus: Square) -> (__m512i, u64) {
    unsafe {
        let perm = _mm512_loadu_si512(RAY_PERMUTATIONS.get_unchecked(focus as usize).as_ptr().cast());
        let mask = _mm512_testn_epi8_mask(perm, _mm512_set1_epi8(0x80u8 as i8));
        (perm, mask)
    }
}

pub fn closest_on_rays(rays: __m512i) -> u64 {
    let occupied = unsafe { _mm512_test_epi8_mask(rays, rays) };
    let o = occupied | 0x8181818181818181;
    let x = o ^ (o - 0x0303030303030303);
    x & occupied
}

pub fn ray_fill(x: u64) -> u64 {
    let x = (x + 0x7E7E7E7E7E7E7E7E) & 0x8080808080808080;
    x - (x >> 7)
}

pub fn exclude_square(board: __m512i, sq: Square) -> __m512i {
    unsafe { _mm512_mask_blend_epi8(sq.to_bb().0, board, _mm512_set1_epi8(Piece::None as i8)) }
}

pub fn board_to_rays(perm: __m512i, valid: u64, board: __m512i) -> (__m512i, __m512i) {
    unsafe {
        let lut = _mm_loadu_si128(PIECE_TO_BIT_TABLE.as_ptr().cast());
        let pboard = _mm512_permutexvar_epi8(perm, board);
        let rays = _mm512_maskz_shuffle_epi8(valid, _mm512_broadcast_i32x4(lut), pboard);
        (pboard, rays)
    }
}

pub fn attackers_along_rays(rays: __m512i) -> u64 {
    unsafe {
        let mask = _mm512_loadu_si512(RAY_ATTACKERS_MASK.as_ptr().cast());
        _mm512_test_epi8_mask(rays, mask)
    }
}

pub fn attacking_along_rays(piece: Piece, occupied: u64) -> u64 {
    RAY_ATTACKS_MASK[piece as usize] & occupied
}

pub fn sliders_along_rays(rays: __m512i) -> u64 {
    unsafe {
        let mask = _mm512_loadu_si512(RAY_SLIDERS_MASK.as_ptr().cast());
        _mm512_test_epi8_mask(rays, mask) & 0xFEFEFEFEFEFEFEFE
    }
}

#[allow(clippy::too_many_arguments)]
pub fn splat_threats(
    accum: &mut ThreatAccumulator, pboard: __m512i, perm: __m512i, attacked: u64, attackers: u64, focus_piece: Piece,
    focus_sq: Square, add: bool,
) {
    use std::arch::x86_64::*;

    unsafe {
        let add = (add as u32) << 31;
        let add = _mm512_set1_epi32(add as i32);

        let focus_pair = {
            let pair = focus_piece as u16 | ((focus_sq as u16) << 8);
            _mm512_set1_epi16(pair as i16)
        };

        // Maximum 8 attacked, Maximum 16 attackers.
        let attacked_pieces = _mm512_castsi512_si256(_mm512_maskz_compress_epi8(attacked, pboard));
        let attacked_squares = _mm512_castsi512_si256(_mm512_maskz_compress_epi8(attacked, perm));
        let attackers_pieces = _mm512_maskz_compress_epi8(attackers, pboard);
        let attackers_squares = _mm512_maskz_compress_epi8(attackers, perm);

        let attacked_idx = _mm256_set_epi8(
            39, 7, 39, 7, 38, 6, 38, 6, 37, 5, 37, 5, 36, 4, 36, 4, 35, 3, 35, 3, 34, 2, 34, 2, 33, 1, 33, 1, 32, 0,
            32, 0,
        );
        let attackers_idx = _mm512_set_epi8(
            79, 15, 79, 15, 78, 14, 78, 14, 77, 13, 77, 13, 76, 12, 76, 12, 75, 11, 75, 11, 74, 10, 74, 10, 73, 9, 73,
            9, 72, 8, 72, 8, 71, 7, 71, 7, 70, 6, 70, 6, 69, 5, 69, 5, 68, 4, 68, 4, 67, 3, 67, 3, 66, 2, 66, 2, 65, 1,
            65, 1, 64, 0, 64, 0,
        );

        let attacked_pairs = _mm256_permutex2var_epi8(attacked_pieces, attacked_idx, attacked_squares);
        let attackers_pairs = _mm512_permutex2var_epi8(attackers_pieces, attackers_idx, attackers_squares);

        let attacked_vector = _mm256_or_si256(
            _mm256_mask_mov_epi8(_mm512_castsi512_si256(focus_pair), 0xCCCCCCCC, attacked_pairs),
            _mm512_castsi512_si256(add),
        );
        let attackers_vector =
            _mm512_or_si512(_mm512_mask_mov_epi8(focus_pair, 0x3333333333333333, attackers_pairs), add);

        accum.delta.unchecked_write(|data| {
            _mm256_storeu_si256(data.cast(), attacked_vector);
            attacked.count_ones() as usize
        });
        accum.delta.unchecked_write(|data| {
            _mm512_storeu_si512(data.cast(), attackers_vector);
            attackers.count_ones() as usize
        });
    }
}

pub fn splat_xray_threats(
    accum: &mut ThreatAccumulator, pboard: __m512i, perm: __m512i, sliders: u64, victim_mask: u64, add: bool,
) {
    // Deal with x-rays
    unsafe {
        let add = (add as u32) << 31;
        let add = _mm_set1_epi32(add as i32);

        unsafe fn flip_rays(x: __m512i) -> __m512i {
            _mm512_shuffle_i64x2(x, x, 0b01001110)
        }

        unsafe fn compress(m: u64, v: __m512i) -> __m128i {
            _mm512_castsi512_si128(_mm512_maskz_compress_epi8(m, v))
        }

        let p1 = compress(sliders, pboard);
        let sq1 = compress(sliders, perm);
        let p2 = compress(victim_mask, flip_rays(pboard));
        let sq2 = compress(victim_mask, flip_rays(perm));

        let pair1 = _mm_unpacklo_epi8(p1, sq1);
        let pair2 = _mm_unpacklo_epi8(p2, sq2);

        accum.delta.unchecked_write(|data| {
            _mm_storeu_si128(data.cast(), _mm_or_si128(_mm_unpacklo_epi16(pair1, pair2), add));
            _mm_storeu_si128(data.add(4).cast(), _mm_or_si128(_mm_unpackhi_epi16(pair1, pair2), add));
            debug_assert_eq!(sliders.count_ones(), victim_mask.count_ones());
            sliders.count_ones() as usize
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn splat_xray_threats2(
    accum: &mut ThreatAccumulator, pboard_a: __m512i, perm_a: __m512i, sliders_a: u64, victims_a: u64, add_a: bool,
    pboard_b: __m512i, perm_b: __m512i, sliders_b: u64, victims_b: u64, add_b: bool,
) {
    // Deal with x-rays
    unsafe {
        let add_a = (add_a as u32) << 31;
        let add_b = (add_b as u32) << 31;
        let add_a = _mm_set1_epi32(add_a as i32);
        let add_b = _mm_set1_epi32(add_b as i32);

        unsafe fn flip_rays(x: __m512i) -> __m512i {
            _mm512_shuffle_i64x2(x, x, 0b01001110)
        }

        unsafe fn compress(m: u64, v: __m512i) -> __m128i {
            _mm512_castsi512_si128(_mm512_maskz_compress_epi8(m, v))
        }

        let p1_a = compress(sliders_a, pboard_a);
        let p1_b = compress(sliders_b, pboard_b);
        let sq1_a = compress(sliders_a, perm_a);
        let sq1_b = compress(sliders_b, perm_b);
        let p2_a = compress(victims_a, flip_rays(pboard_a));
        let p2_b = compress(victims_b, flip_rays(pboard_b));
        let sq2_a = compress(victims_a, flip_rays(perm_a));
        let sq2_b = compress(victims_b, flip_rays(perm_b));

        let pair1_a = _mm_unpacklo_epi8(p1_a, sq1_a);
        let pair1_b = _mm_unpacklo_epi8(p1_b, sq1_b);
        let pair2_a = _mm_unpacklo_epi8(p2_a, sq2_a);
        let pair2_b = _mm_unpacklo_epi8(p2_b, sq2_b);

        let vec1_a = _mm_or_si128(_mm_unpacklo_epi16(pair1_a, pair2_a), add_a);
        let vec1_b = _mm_or_si128(_mm_unpacklo_epi16(pair1_b, pair2_b), add_b);
        let vec2_a = _mm_or_si128(_mm_unpackhi_epi16(pair1_a, pair2_a), add_a);
        let vec2_b = _mm_or_si128(_mm_unpackhi_epi16(pair1_b, pair2_b), add_b);

        accum.delta.unchecked_write(|data| {
            _mm_storeu_si128(data.cast(), vec1_a);
            _mm_storeu_si128(data.add(4).cast(), vec2_a);
            debug_assert_eq!(sliders_a.count_ones(), victims_a.count_ones());
            sliders_a.count_ones() as usize
        });
        accum.delta.unchecked_write(|data| {
            _mm_storeu_si128(data.cast(), vec1_b);
            _mm_storeu_si128(data.add(4).cast(), vec2_b);
            debug_assert_eq!(sliders_b.count_ones(), victims_b.count_ones());
            sliders_b.count_ones() as usize
        });
    }
}
