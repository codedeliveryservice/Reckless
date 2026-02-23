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
    accum: &mut ThreatAccumulator, is_to: bool, pboard: __m512i, perm: __m512i, bitray: u64, p2: Piece, sq2: Square,
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

        let iota = _mm512_maskz_compress_epi8(bitray, perm);
        let mailbox = _mm512_maskz_compress_epi8(bitray, pboard);

        let idx = _mm512_set_epi8(
            79, 15, 79, 15, 78, 14, 78, 14, 77, 13, 77, 13, 76, 12, 76, 12, 75, 11, 75, 11, 74, 10, 74, 10, 73, 9, 73,
            9, 72, 8, 72, 8, 71, 7, 71, 7, 70, 6, 70, 6, 69, 5, 69, 5, 68, 4, 68, 4, 67, 3, 67, 3, 66, 2, 66, 2, 65, 1,
            65, 1, 64, 0, 64, 0,
        );

        let widen = _mm512_permutex2var_epi8(mailbox, idx, iota);
        let mask = if is_to { 0xCCCCCCCCCCCCCCCC } else { 0x3333333333333333 };

        let vector = _mm512_or_si512(_mm512_mask_mov_epi8(template, mask, widen), add);

        accum.delta.unchecked_write(|data| {
            _mm512_storeu_si512(data.cast(), vector);
            bitray.count_ones() as usize
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
