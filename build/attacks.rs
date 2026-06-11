//! Contains functions for generating attack masks on the fly. Directions are calculated
//! by left or right shift with an appropriate shift mask to avoid wrapping A/H files.
//! In the case of a 1st/8th rank wrapping, the bit is pruned after the shift,
//! so no mask is required.

#![allow(clippy::precedence)]

const A_FILE: u64 = 0x0101010101010101;
const H_FILE: u64 = A_FILE << 7;

const FILE_B: i8 = 1;
const FILE_H: i8 = 7;

const KING_DIRS: [i8; 8] = [7, 8, 9, 1, -7, -8, -9, -1];
const KNIGHT_STEP1: [i8; 4] = [7, 9, -7, -9];
const KNIGHT_STEP2: [i8; 4] = [8, 1, -8, -1];

pub enum Color {
    White,
    Black,
}

#[inline(always)]
pub const fn shift_dir(mut bb: u64, dir: i8) -> u64 {
    let file_offset = dir & 7;

    if file_offset == FILE_B {
        bb &= !H_FILE;
    } else if file_offset == FILE_H {
        bb &= !A_FILE;
    }

    if dir < 0 {
        bb >> (-dir as u32)
    } else {
        bb << (dir as u32)
    }
}

#[inline(always)]
pub fn shift_dirs(bb: u64, dirs: &[i8]) -> u64 {
    let mut targets = 0;

    let mut i = 0;
    while i < dirs.len() {
        targets |= shift_dir(bb, dirs[i]);
        i += 1;
    }

    targets
}

#[inline(always)]
pub fn pawn_attacks(square: u8, color: Color) -> u64 {
    let bb = 1u64 << square;

    match color {
        Color::White => {
            shift_dir(bb, 7)
                | shift_dir(bb, 9)
        }
        Color::Black => {
            shift_dir(bb, -7)
                | shift_dir(bb, -9)
        }
    }
}

#[inline(always)]
pub fn king_attacks(square: u8) -> u64 {
    let bb = 1u64 << square;

    shift_dir(bb, 7)
        | shift_dir(bb, 8)
        | shift_dir(bb, 9)
        | shift_dir(bb, 1)
        | shift_dir(bb, -7)
        | shift_dir(bb, -8)
        | shift_dir(bb, -9)
        | shift_dir(bb, -1)
}

#[inline(always)]
pub fn knight_attacks(square: u8) -> u64 {
    let bb = 1u64 << square;

    let targets =
        shift_dir(bb, 7)
        | shift_dir(bb, 9)
        | shift_dir(bb, -7)
        | shift_dir(bb, -9);

    let attacks =
        shift_dir(targets, 8)
        | shift_dir(targets, 1)
        | shift_dir(targets, -8)
        | shift_dir(targets, -1);

    attacks & !king_attacks(square)
}

pub fn sliding_attacks(square: u8, occupancies: u64, directions: &[i8]) -> u64 {
    let mut attacks = 0;

    for &direction in directions {
        attacks |= generate_slide(square, occupancies, direction);
    }

    attacks
}

pub fn generate_ray(square1: u8, square2: u8, between: bool) -> u64 {
    let mut slide = 0;

    for dir in [8, 9, 1, -7, -8, -9, -1, 7] {
        let ray = generate_slide(square2, 0, dir);

        if ray & (1u64 << square1) != 0 {
            slide = if between {
                ray & generate_slide(square1, 0, -dir)
            } else {
                ray
            };

            break;
        }
    }

    slide
}

#[inline(always)]
fn generate_slide(square: u8, occupancies: u64, direction: i8) -> u64 {
    let mut attacks = 0;
    let mut current = shift_dir(1u64 << square, direction);

    while current != 0 {
        attacks |= current;

        if current & occupancies != 0 {
            break;
        }

        current = shift_dir(current, direction);
    }

    attacks
}