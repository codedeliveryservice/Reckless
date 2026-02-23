use crate::{
    board::Board,
    nnue::ThreatAccumulator,
    types::{Piece, Square},
};

#[cfg(target_feature = "avx512vbmi")]
mod avx512;
#[cfg(target_feature = "avx512vbmi")]
use avx512::*;

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi")))]
mod avx2;
#[cfg(all(target_feature = "avx2", not(target_feature = "avx512vbmi")))]
use avx2::*;

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
    let board = unsafe { board.mailbox_vector() };

    let (perm, valid) = ray_permutation(square);
    let (pboard, rays) = board_to_rays(perm, valid, board);

    let closest = closest_on_rays(rays);
    let attacked = attacking_along_rays(piece, closest);
    let attackers = attackers_along_rays(rays) & closest;
    let sliders = sliders_along_rays(rays) & closest;

    splat_threats(accum, pboard, perm, attacked, attackers, piece, square, add);

    let victim = (closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
    let xray_valid = ray_fill(victim) & ray_fill(sliders);
    splat_xray_threats(accum, pboard, perm, sliders & xray_valid, victim & xray_valid, !add);
}

pub fn push_threats_on_move(accum: &mut ThreatAccumulator, board: &Board, piece: Piece, src: Square, dst: Square) {
    let board = unsafe { board.mailbox_vector() };

    let (src_perm, src_valid) = ray_permutation(src);
    let (dst_perm, dst_valid) = ray_permutation(dst);
    let (src_pboard, src_rays) = board_to_rays(src_perm, src_valid, exclude_square(board, dst));
    let (dst_pboard, dst_rays) = board_to_rays(dst_perm, dst_valid, board);

    let src_closest = closest_on_rays(src_rays);
    let dst_closest = closest_on_rays(dst_rays);
    let src_attacked = attacking_along_rays(piece, src_closest);
    let dst_attacked = attacking_along_rays(piece, dst_closest);
    let src_attackers = attackers_along_rays(src_rays) & src_closest;
    let dst_attackers = attackers_along_rays(dst_rays) & dst_closest;
    let src_sliders = sliders_along_rays(src_rays) & src_closest;
    let dst_sliders = sliders_along_rays(dst_rays) & dst_closest;

    splat_threats(accum, src_pboard, src_perm, src_attacked, src_attackers, piece, src, false);
    splat_threats(accum, dst_pboard, dst_perm, dst_attacked, dst_attackers, piece, dst, true);

    let src_victim = (src_closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
    let dst_victim = (dst_closest & 0xFEFEFEFEFEFEFEFE).rotate_right(32);
    let src_xray_valid = ray_fill(src_victim) & ray_fill(src_sliders);
    let dst_xray_valid = ray_fill(dst_victim) & ray_fill(dst_sliders);

    splat_xray_threats2(
        accum,
        src_pboard,
        src_perm,
        src_sliders & src_xray_valid,
        src_victim & src_xray_valid,
        true,
        dst_pboard,
        dst_perm,
        dst_sliders & dst_xray_valid,
        dst_victim & dst_xray_valid,
        false,
    );
}

pub fn push_threats_on_mutate(
    accum: &mut ThreatAccumulator, board: &Board, old_piece: Piece, new_piece: Piece, square: Square,
) {
    let board = unsafe { board.mailbox_vector() };

    let (perm, valid) = ray_permutation(square);
    let (pboard, rays) = board_to_rays(perm, valid, board);

    let closest = closest_on_rays(rays);
    let old_attacked = attacking_along_rays(old_piece, closest);
    let new_attacked = attacking_along_rays(new_piece, closest);
    let attackers = attackers_along_rays(rays) & closest;

    splat_threats(accum, pboard, perm, old_attacked, attackers, old_piece, square, false);
    splat_threats(accum, pboard, perm, new_attacked, attackers, new_piece, square, true);
}
