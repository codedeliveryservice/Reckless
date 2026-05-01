//! Contains functions for generating attack masks on the fly. Directions are calculated
//! by left or right shift with an appropriate shift mask to avoid wrapping A/H files.
//! In the case of a 1st/8th rank wrapping, the bit is pruned after the shift,
//! so no mask is required.

#![allow(clippy::precedence)]

const A_FILE: u64 = 0x101010101010101;
const H_FILE: u64 = A_FILE << 7;
const FILE_B: i8 = 1;
const FILE_H: i8 = 7;

pub enum Color {
    White,
    Black,
}

pub fn shift_dir(mut bb: u64, dir: i8) -> u64 {
    let file_offset = dir & 0x7;

    if file_offset == FILE_B { bb &= !H_FILE; }
    if file_offset == FILE_H { bb &= !A_FILE; }

    if dir < 0 { bb >> -dir } else { bb << dir }
}

pub fn shift_dirs(bb: u64, dirs: &[i8]) -> u64 {
    let mut targets = 0;
    for dir in dirs {
        targets |= shift_dir(bb, *dir);
    }
    targets
}

pub fn pawn_attacks(square: u8, color: Color) -> u64 {
    if matches!(color, Color::White) {
        shift_dirs(1 << square, &[7, 9])
    } else {
        shift_dirs(1 << square, &[-7, -9])
    }
}

pub fn king_attacks(square: u8) -> u64 {
    shift_dirs(1 << square, &[7, 8, 9, 1, -7, -8, -9, -1])
}

pub fn knight_attacks(square: u8) -> u64 {
    let targets = shift_dirs(1 << square, &[7, 9, -7, -9]);
    let targets = shift_dirs(targets, &[8, 1, -8, -1]);
    targets & !king_attacks(square)
}

pub fn sliding_attacks(square: u8, occupancies: u64, directions: &[(i8, i8)]) -> u64 {
    directions.iter().fold(0, |output, &direction| output | generate_sliding_attacks(square, occupancies, direction))
}

fn generate_sliding_attacks(square: u8, occupancies: u64, direction: (i8, i8)) -> u64 {
    let mut output = 0;

    let mut rank = (square / 8) as i8 + direction.0;
    let mut file = (square % 8) as i8 + direction.1;

    while (0..8).contains(&file) && (0..8).contains(&rank) {
        let bitboard = 1 << (rank * 8 + file);
        output |= bitboard;

        if (bitboard & occupancies) != 0 {
            break;
        }

        rank += direction.0;
        file += direction.1;
    }

    output
}
