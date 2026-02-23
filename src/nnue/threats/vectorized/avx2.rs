use std::arch::x86_64::*;

use crate::{
    nnue::{
        accumulator::{ThreatAccumulator, ThreatDelta},
        threats::vectorized::{
            PIECE_TO_BIT_TABLE, RAY_ATTACKERS_MASK, RAY_ATTACKS_MASK, RAY_PERMUTATIONS, RAY_SLIDERS_MASK,
        },
    },
    types::{Piece, Square},
};

unsafe fn loadu(ptr: *const __m256i) -> [__m256i; 2] {
    [_mm256_loadu_si256(ptr), _mm256_loadu_si256(ptr.add(1))]
}

unsafe fn to_u64(vector: [__m256i; 2]) -> u64 {
    _mm256_movemask_epi8(vector[0]) as u32 as u64 | ((_mm256_movemask_epi8(vector[1]) as u64) << 32)
}

pub fn ray_permutation(focus: Square) -> ([__m256i; 2], [__m256i; 2]) {
    unsafe {
        let perm = loadu(RAY_PERMUTATIONS.get_unchecked(focus as usize).as_ptr().cast());
        let mask = [
            _mm256_cmpeq_epi8(perm[0], _mm256_set1_epi8(0x80u8 as i8)),
            _mm256_cmpeq_epi8(perm[1], _mm256_set1_epi8(0x80u8 as i8)),
        ];
        (perm, mask)
    }
}

pub fn closest_on_rays(rays: [__m256i; 2]) -> u64 {
    let occupied = unsafe {
        !to_u64([
            _mm256_cmpeq_epi8(rays[0], _mm256_setzero_si256()),
            _mm256_cmpeq_epi8(rays[1], _mm256_setzero_si256()),
        ])
    };
    let o = occupied | 0x8181818181818181;
    let x = o ^ (o - 0x0303030303030303);
    x & occupied
}

pub fn ray_fill(x: u64) -> u64 {
    let x = (x + 0x7E7E7E7E7E7E7E7E) & 0x8080808080808080;
    x - (x >> 7)
}

pub fn exclude_square(board: [__m256i; 2], sq: Square) -> [__m256i; 2] {
    unsafe {
        let iota = [
            _mm256_set_epi8(
                31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5,
                4, 3, 2, 1, 0,
            ),
            _mm256_set_epi8(
                63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39, 38,
                37, 36, 35, 34, 33, 32,
            ),
        ];
        let none = _mm256_set1_epi8(Piece::None as i8);
        let sq = _mm256_set1_epi8(sq as i8);
        [
            _mm256_blendv_epi8(board[0], none, _mm256_cmpeq_epi8(iota[0], sq)),
            _mm256_blendv_epi8(board[1], none, _mm256_cmpeq_epi8(iota[1], sq)),
        ]
    }
}

pub fn board_to_rays(perm: [__m256i; 2], invalid: [__m256i; 2], board: [__m256i; 2]) -> ([__m256i; 2], [__m256i; 2]) {
    unsafe {
        let half_swizzler = |bytes0: __m256i, bytes1: __m256i, idxs: __m256i| {
            let mask0 = _mm256_slli_epi64(idxs, 2);
            let mask1 = _mm256_slli_epi64(idxs, 3);

            let lolo0 = _mm256_shuffle_epi8(_mm256_permute2x128_si256::<0x00>(bytes0, bytes0), idxs);
            let hihi0 = _mm256_shuffle_epi8(_mm256_permute2x128_si256::<0x11>(bytes0, bytes0), idxs);
            let x = _mm256_blendv_epi8(lolo0, hihi0, mask1);

            let lolo1 = _mm256_shuffle_epi8(_mm256_permute2x128_si256::<0x00>(bytes1, bytes1), idxs);
            let hihi1 = _mm256_shuffle_epi8(_mm256_permute2x128_si256::<0x11>(bytes1, bytes1), idxs);
            let y = _mm256_blendv_epi8(lolo1, hihi1, mask1);

            _mm256_blendv_epi8(x, y, mask0)
        };

        let lut = _mm256_broadcastsi128_si256(_mm_loadu_si128(PIECE_TO_BIT_TABLE.as_ptr().cast()));
        let pboard = [half_swizzler(board[0], board[1], perm[0]), half_swizzler(board[0], board[1], perm[1])];
        let rays = [
            _mm256_andnot_si256(invalid[0], _mm256_shuffle_epi8(lut, pboard[0])),
            _mm256_andnot_si256(invalid[1], _mm256_shuffle_epi8(lut, pboard[1])),
        ];
        (pboard, rays)
    }
}

pub fn attackers_along_rays(rays: [__m256i; 2]) -> u64 {
    unsafe {
        let mask = loadu(RAY_ATTACKERS_MASK.as_ptr().cast());
        !to_u64([
            _mm256_cmpeq_epi8(_mm256_and_si256(rays[0], mask[0]), _mm256_setzero_si256()),
            _mm256_cmpeq_epi8(_mm256_and_si256(rays[1], mask[1]), _mm256_setzero_si256()),
        ])
    }
}

pub fn attacking_along_rays(piece: Piece, occupied: u64) -> u64 {
    unsafe { *RAY_ATTACKS_MASK.get_unchecked(piece as usize) & occupied }
}

pub fn sliders_along_rays(rays: [__m256i; 2]) -> u64 {
    unsafe {
        let mask = loadu(RAY_SLIDERS_MASK.as_ptr().cast());
        !to_u64([
            _mm256_cmpeq_epi8(_mm256_and_si256(rays[0], mask[0]), _mm256_setzero_si256()),
            _mm256_cmpeq_epi8(_mm256_and_si256(rays[1], mask[1]), _mm256_setzero_si256()),
        ]) & 0xFEFEFEFEFEFEFEFE
    }
}

#[allow(clippy::too_many_arguments)]
pub fn splat_threats(
    accum: &mut ThreatAccumulator, pboard: [__m256i; 2], perm: [__m256i; 2], mut attacked: u64, mut attackers: u64,
    focus_piece: Piece, focus_sq: Square, add: bool,
) {
    let pieces = unsafe { std::mem::transmute::<[__m256i; 2], [Piece; 64]>(pboard) };
    let squares = unsafe { std::mem::transmute::<[__m256i; 2], [Square; 64]>(perm) };

    while attacked != 0 {
        let i = attacked.trailing_zeros() as usize;
        // SAFETY: i is always less than 64
        let piece = unsafe { pieces.get_unchecked(i) };
        let square = unsafe { squares.get_unchecked(i) };
        accum.delta.push(ThreatDelta::new(focus_piece, focus_sq, *piece, *square, add));
        attacked &= attacked - 1;
    }

    while attackers != 0 {
        let i = attackers.trailing_zeros() as usize;
        // SAFETY: i is always less than 64
        let piece = unsafe { pieces.get_unchecked(i) };
        let square = unsafe { squares.get_unchecked(i) };
        accum.delta.push(ThreatDelta::new(*piece, *square, focus_piece, focus_sq, add));
        attackers &= attackers - 1;
    }
}

pub fn splat_xray_threats(
    accum: &mut ThreatAccumulator, pboard: [__m256i; 2], perm: [__m256i; 2], mut sliders: u64, mut victims: u64,
    add: bool,
) {
    debug_assert_eq!(sliders.count_ones(), victims.count_ones());

    let pieces = unsafe { std::mem::transmute::<[__m256i; 2], [Piece; 64]>(pboard) };
    let squares = unsafe { std::mem::transmute::<[__m256i; 2], [Square; 64]>(perm) };

    while sliders != 0 {
        let slider = sliders.trailing_zeros() as usize;
        let victim = victims.trailing_zeros() as usize;

        // SAFETY: slider is always less than 64
        let attacker = unsafe { pieces.get_unchecked(slider) };
        let attacker_sq = unsafe { squares.get_unchecked(slider) };
        // SAFETY: victim is always less than 64
        let attacked = unsafe { pieces.get_unchecked((victim + 32) % 64) };
        let attacked_sq = unsafe { squares.get_unchecked((victim + 32) % 64) };

        accum.delta.push(ThreatDelta::new(*attacker, *attacker_sq, *attacked, *attacked_sq, add));

        sliders &= sliders - 1;
        victims &= victims - 1;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn splat_xray_threats2(
    accum: &mut ThreatAccumulator, pboard_a: [__m256i; 2], perm_a: [__m256i; 2], sliders_a: u64, victims_a: u64,
    add_a: bool, pboard_b: [__m256i; 2], perm_b: [__m256i; 2], sliders_b: u64, victims_b: u64, add_b: bool,
) {
    splat_xray_threats(accum, pboard_a, perm_a, sliders_a, victims_a, add_a);
    splat_xray_threats(accum, pboard_b, perm_b, sliders_b, victims_b, add_b);
}
