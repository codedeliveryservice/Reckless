use std::arch::x86_64::*;

use crate::types::{Piece, Square};

pub fn ray_permutation(focus: Square) -> (__m512i, u64) {
    const PERMS: [[u8; 64]; 64] = {
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

    unsafe {
        let perm = _mm512_loadu_si512(PERMS.get_unchecked(focus as usize).as_ptr().cast());
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
        let lut: [u8; 16] = [
            //   White,      Black,
            0b00000001, 0b00000010, // Pawn
            0b00000100, 0b00000100, // Knight
            0b00001000, 0b00001000, // Bishop
            0b00010000, 0b00010000, // Rook
            0b00100000, 0b00100000, // Queen
            0b01000000, 0b01000000, // King
            0, 0, 0, 0,
        ];
        let lut = _mm_loadu_si128(lut.as_ptr().cast());

        let pboard = _mm512_permutexvar_epi8(perm, board);
        let rays = _mm512_maskz_shuffle_epi8(valid, _mm512_broadcast_i32x4(lut), pboard);
        (pboard, rays)
    }
}

pub fn attackers_along_rays(rays: __m512i) -> u64 {
    unsafe {
        let horse = 0b00000100; // knight
        let orth = 0b00110000; // rook and queen
        let diag = 0b00101000; // bishop and queen
        let ortho_near = 0b01110000; // king, rook and queen
        let wpawn_near = 0b01101001; // wp, king, bishop, queen
        let bpawn_near = 0b01101010; // bp, king, bishop, queen

        let mask: [u8; 64] = [
            horse, ortho_near, orth, orth, orth, orth, orth, orth, // N
            horse, bpawn_near, diag, diag, diag, diag, diag, diag, // NE
            horse, ortho_near, orth, orth, orth, orth, orth, orth, // E
            horse, wpawn_near, diag, diag, diag, diag, diag, diag, // SE
            horse, ortho_near, orth, orth, orth, orth, orth, orth, // S
            horse, wpawn_near, diag, diag, diag, diag, diag, diag, // SW
            horse, ortho_near, orth, orth, orth, orth, orth, orth, // W
            horse, bpawn_near, diag, diag, diag, diag, diag, diag, // NW
        ];
        let mask = _mm512_loadu_si512(mask.as_ptr().cast());

        _mm512_test_epi8_mask(rays, mask)
    }
}

pub fn attacking_along_rays(piece: Piece, occupied: u64) -> u64 {
    let lut: [u64; 12] = [
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

    lut[piece as usize] & occupied
}

pub fn sliders_along_rays(rays: __m512i) -> u64 {
    unsafe {
        let orth = 0b00110000; // rook and queen
        let diag = 0b00101000; // bishop and queen

        let mask: [u8; 64] = [
            0x80, orth, orth, orth, orth, orth, orth, orth, // N
            0x80, diag, diag, diag, diag, diag, diag, diag, // NE
            0x80, orth, orth, orth, orth, orth, orth, orth, // E
            0x80, diag, diag, diag, diag, diag, diag, diag, // SE
            0x80, orth, orth, orth, orth, orth, orth, orth, // S
            0x80, diag, diag, diag, diag, diag, diag, diag, // SW
            0x80, orth, orth, orth, orth, orth, orth, orth, // W
            0x80, diag, diag, diag, diag, diag, diag, diag, // NW
        ];
        let mask = _mm512_loadu_si512(mask.as_ptr().cast());

        _mm512_test_epi8_mask(rays, mask) & 0xFEFEFEFEFEFEFEFE
    }
}

pub fn flip_rays(x: __m512i) -> __m512i {
    unsafe { _mm512_shuffle_i64x2(x, x, 0b01001110) }
}
