//! Contains functions for generating attack masks on the fly. Directions are calculated
//! by left or right shift with an appropriate shift mask to avoid wrapping A/H files.
//! In the case of a 1st/8th rank wrapping, the bit is pruned after the shift,
//! so no mask is required.

#![allow(clippy::precedence)]

const A_FILE: u64 = 0x101010101010101;
const H_FILE: u64 = A_FILE << 7;

pub enum Color {
    White,
    Black,
}

pub const fn shift_left(bb: u64) -> u64 {
    (bb & !A_FILE) >> 1
}

pub const fn shift_right(bb: u64) -> u64 {
    (bb & !H_FILE) << 1
}

pub const fn pawn_attacks(square: u8, color: Color) -> u64 {
    let bb = 1 << square;
    let up = if matches!(color, Color::White) { bb << 8 } else { bb >> 8 };
    shift_left(up) | shift_right(up)
}

pub const fn king_attacks(square: u8) -> u64 {
    let sq_bb = 1 << square;
    let bb = sq_bb | (sq_bb << 8) | (sq_bb >> 8);
    (bb | shift_left(bb) | shift_right(bb)) & !sq_bb
}

pub const fn knight_attacks(square: u8) -> u64 {
    let bb = pawn_attacks(square, Color::White) | pawn_attacks(square, Color::Black);
    let bb = shift_left(bb) | shift_right(bb) | (bb << 8) | (bb >> 8);
    bb & !king_attacks(square)
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
