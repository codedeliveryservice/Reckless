use std::arch::x86_64::*;

use crate::{
    nnue::threats::vectorized::{
        PIECE_TO_BIT_TABLE, RAY_ATTACKERS_MASK, RAY_ATTACKS_MASK, RAY_PERMUTATIONS, RAY_SLIDERS_MASK,
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

pub fn closest_on_rays(occupied: u64) -> u64 {
    let o = occupied | 0x8181818181818181;
    let x = o ^ (o - 0x0303030303030303);
    x & occupied
}

pub fn ray_fill(x: u64) -> u64 {
    let x = (x + 0x7E7E7E7E7E7E7E7E) & 0x8080808080808080;
    x - (x >> 7)
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

pub fn flip_rays(x: __m512i) -> __m512i {
    unsafe { _mm512_shuffle_i64x2(x, x, 0b01001110) }
}
