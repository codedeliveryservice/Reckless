//! Contains functions for generating attack masks on the fly. Directions are calculated
//! by left or right shift with an appropriate shift mask to avoid wrapping A/H files.
//! In the case of a 1st/8th rank wrapping, the bit is pruned after the shift,
//! so no mask is required.

const A_FILE: u64 = 0x101010101010101;
const B_FILE: u64 = A_FILE << 1;
const H_FILE: u64 = A_FILE << 7;
const G_FILE: u64 = A_FILE << 6;

const AB_FILE: u64 = A_FILE | B_FILE;
const GH_FILE: u64 = G_FILE | H_FILE;

pub enum Color {
    White,
    Black,
}

pub fn pawn_attacks(square: u8, color: Color) -> u64 {
    let bitboard = 1 << square;
    if matches!(color, Color::White) {
        (bitboard & !A_FILE) << 7 | (bitboard & !H_FILE) << 9
    } else {
        (bitboard & !H_FILE) >> 7 | (bitboard & !A_FILE) >> 9
    }
}

pub fn king_attacks(square: u8) -> u64 {
    let bitboard = 1 << square;

    (bitboard >> 8 | bitboard << 8)
        | (bitboard & !A_FILE) >> 9
        | (bitboard & !A_FILE) >> 1
        | (bitboard & !A_FILE) << 7
        | (bitboard & !H_FILE) >> 7
        | (bitboard & !H_FILE) << 1
        | (bitboard & !H_FILE) << 9
}

pub fn knight_attacks(square: u8) -> u64 {
    let bitboard = 1 << square;

    (bitboard & !A_FILE) >> 17
        | (bitboard & !A_FILE) << 15
        | (bitboard & !H_FILE) >> 15
        | (bitboard & !H_FILE) << 17
        | (bitboard & !AB_FILE) >> 10
        | (bitboard & !AB_FILE) << 6
        | (bitboard & !GH_FILE) >> 6
        | (bitboard & !GH_FILE) << 10
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
