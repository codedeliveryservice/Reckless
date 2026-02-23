use crate::{
    board::Board,
    nnue::ThreatAccumulator,
    types::{Piece, Square},
};

#[cfg(target_feature = "avx512vbmi")]
mod avx512;
#[cfg(target_feature = "avx512vbmi")]
pub use avx512::*;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi")))]
mod avx2;
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi")))]
pub use avx2::*;

const RAY_PERMUTATIONS: [[u8; 64]; 64] = {
    const OFFSETS: [u8; 64] = [
        0x1F, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, // N
        0x21, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, // NE
        0x12, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // E
        0xF2, 0xF1, 0xE2, 0xD3, 0xC4, 0xB5, 0xA6, 0x97, // SE
        0xE1, 0xF0, 0xE0, 0xD0, 0xC0, 0xB0, 0xA0, 0x90, // S
        0xDF, 0xEF, 0xDE, 0xCD, 0xBC, 0xAB, 0x9A, 0x89, // SW
        0xEE, 0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, // W
        0x0E, 0x0F, 0x1E, 0x2D, 0x3C, 0x4B, 0x5A, 0x69, // NW
    ];

    let mut perms = [[0u8; 64]; 64];
    let mut sq = 0;
    while sq < 64 {
        let focus = sq as u8;
        let focus = focus + (focus & 0x38);
        let mut i = 0;
        while i < 64 {
            let wide_result = OFFSETS[i].wrapping_add(focus);
            let valid = wide_result & 0x88 == 0;
            let narrow_result = ((wide_result & 0x70) >> 1) + (wide_result & 0x07);
            perms[sq][i] = if valid { narrow_result } else { 0x80 };
            i += 1;
        }
        sq += 1;
    }
    perms
};

const RAY_ATTACKS_MASK: [u64; 12] = [
    0x02_00_00_00_00_00_02_00, // WhitePawn
    0x00_00_02_00_02_00_00_00, // BlackPawn
    0x01_01_01_01_01_01_01_01, // WhiteKnight
    0x01_01_01_01_01_01_01_01, // BlackKnight
    0xFE_00_FE_00_FE_00_FE_00, // WhiteBishop
    0xFE_00_FE_00_FE_00_FE_00, // BlackBishop
    0x00_FE_00_FE_00_FE_00_FE, // WhiteRook
    0x00_FE_00_FE_00_FE_00_FE, // BlackRook
    0xFE_FE_FE_FE_FE_FE_FE_FE, // WhiteQueen
    0xFE_FE_FE_FE_FE_FE_FE_FE, // BlackQueen
    0x02_02_02_02_02_02_02_02, // WhiteKing
    0x02_02_02_02_02_02_02_02, // BlackKing
];

const PIECE_TO_BIT_TABLE: [u8; 16] = [
    //   White,      Black,
    0b00000001, 0b00000010, // Pawn
    0b00000100, 0b00000100, // Knight
    0b00001000, 0b00001000, // Bishop
    0b00010000, 0b00010000, // Rook
    0b00100000, 0b00100000, // Queen
    0b01000000, 0b01000000, // King
    0, 0, 0, 0,
];

const RAY_ATTACKERS_MASK: [u8; 64] = {
    let horse = 0b00000100; // knight
    let orth = 0b00110000; // rook and queen
    let diag = 0b00101000; // bishop and queen
    let ortho_near = 0b01110000; // king, rook and queen
    let wpawn_near = 0b01101001; // wp, king, bishop, queen
    let bpawn_near = 0b01101010; // bp, king, bishop, queen

    [
        horse, ortho_near, orth, orth, orth, orth, orth, orth, // N
        horse, bpawn_near, diag, diag, diag, diag, diag, diag, // NE
        horse, ortho_near, orth, orth, orth, orth, orth, orth, // E
        horse, wpawn_near, diag, diag, diag, diag, diag, diag, // SE
        horse, ortho_near, orth, orth, orth, orth, orth, orth, // S
        horse, wpawn_near, diag, diag, diag, diag, diag, diag, // SW
        horse, ortho_near, orth, orth, orth, orth, orth, orth, // W
        horse, bpawn_near, diag, diag, diag, diag, diag, diag, // NW
    ]
};

const RAY_SLIDERS_MASK: [u8; 64] = {
    let orth = 0b00110000; // rook and queen
    let diag = 0b00101000; // bishop and queen

    [
        0x80, orth, orth, orth, orth, orth, orth, orth, // N
        0x80, diag, diag, diag, diag, diag, diag, diag, // NE
        0x80, orth, orth, orth, orth, orth, orth, orth, // E
        0x80, diag, diag, diag, diag, diag, diag, diag, // SE
        0x80, orth, orth, orth, orth, orth, orth, orth, // S
        0x80, diag, diag, diag, diag, diag, diag, diag, // SW
        0x80, orth, orth, orth, orth, orth, orth, orth, // W
        0x80, diag, diag, diag, diag, diag, diag, diag, // NW
    ]
};

pub fn push_threats_on_change(accum: &mut ThreatAccumulator, board: &Board, piece: Piece, square: Square, add: bool) {
    use std::arch::x86_64::*;

    let (perm, valid) = ray_permutation(square);
    let (pboard, rays) = board_to_rays(perm, valid, unsafe { board.mailbox_vector() });
    let occupied = unsafe { _mm512_test_epi8_mask(rays, rays) };

    let closest = closest_on_rays(occupied);
    let attacked = attacking_along_rays(piece, closest);
    let attackers = attackers_along_rays(rays) & closest;
    let sliders = sliders_along_rays(rays) & closest;

    splat_threats(accum, true, pboard, perm, attacked, piece, square, add);
    splat_threats(accum, false, pboard, perm, attackers, piece, square, add);

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
        let p2 = compress(victim_mask & xray_valid, flip_rays(pboard));
        let sq2 = compress(victim_mask & xray_valid, flip_rays(perm));

        let pair1 = _mm_unpacklo_epi8(p1, sq1);
        let pair2 = _mm_unpacklo_epi8(p2, sq2);

        accum.delta.unchecked_write(|data| {
            _mm_storeu_si128(data.cast(), _mm_or_si128(_mm_unpacklo_epi16(pair1, pair2), nadd));
            _mm_storeu_si128(data.add(4).cast(), _mm_or_si128(_mm_unpackhi_epi16(pair1, pair2), nadd));
            (sliders & xray_valid).count_ones() as usize
        });
    }
}

pub fn push_threats_on_move(accum: &mut ThreatAccumulator, board: &Board, piece: Piece, src: Square, dst: Square) {
    use std::arch::x86_64::*;

    let board =
        unsafe { _mm512_mask_blend_epi8(dst.to_bb().0, board.mailbox_vector(), _mm512_set1_epi8(Piece::None as i8)) };

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

    splat_threats(accum, true, src_pboard, src_perm, src_attacked, piece, src, false);
    splat_threats(accum, false, src_pboard, src_perm, src_attackers, piece, src, false);
    splat_threats(accum, true, dst_pboard, dst_perm, dst_attacked, piece, dst, true);
    splat_threats(accum, false, dst_pboard, dst_perm, dst_attackers, piece, dst, true);

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

        accum.delta.unchecked_write(|data| {
            let add = _mm_set1_epi32(0x80000000u32 as i32);
            _mm_storeu_si128(data.cast(), _mm_or_si128(_mm_unpacklo_epi16(src_pair1, src_pair2), add));
            _mm_storeu_si128(data.add(4).cast(), _mm_or_si128(_mm_unpackhi_epi16(src_pair1, src_pair2), add));
            (src_sliders & src_xray_valid).count_ones() as usize
        });
        accum.delta.unchecked_write(|data| {
            _mm_storeu_si128(data.cast(), _mm_unpacklo_epi16(dst_pair1, dst_pair2));
            _mm_storeu_si128(data.add(4).cast(), _mm_unpackhi_epi16(dst_pair1, dst_pair2));
            (dst_sliders & dst_xray_valid).count_ones() as usize
        });
    }
}

pub fn push_threats_on_mutate(
    accum: &mut ThreatAccumulator, board: &Board, old_piece: Piece, new_piece: Piece, square: Square,
) {
    use std::arch::x86_64::*;

    let (perm, valid) = ray_permutation(square);
    let (pboard, rays) = board_to_rays(perm, valid, unsafe { board.mailbox_vector() });
    let occupied = unsafe { _mm512_test_epi8_mask(rays, rays) };

    let closest = closest_on_rays(occupied);
    let old_attacked = attacking_along_rays(old_piece, closest);
    let new_attacked = attacking_along_rays(new_piece, closest);
    let attackers = attackers_along_rays(rays) & closest;

    splat_threats(accum, true, pboard, perm, old_attacked, old_piece, square, false);
    splat_threats(accum, false, pboard, perm, attackers, old_piece, square, false);
    splat_threats(accum, true, pboard, perm, new_attacked, new_piece, square, true);
    splat_threats(accum, false, pboard, perm, attackers, new_piece, square, true);
}

#[allow(clippy::too_many_arguments)]
fn splat_threats(
    accum: &mut ThreatAccumulator, is_to: bool, pboard: std::arch::x86_64::__m512i, perm: std::arch::x86_64::__m512i,
    bitray: u64, p2: Piece, sq2: Square, add: bool,
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
