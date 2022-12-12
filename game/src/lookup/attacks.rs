//! Contains functions for generating attack masks on the fly. Directions are calculated
//! by left or right shift with an appropriate shift mask to avoid wrapping A/H files.
//! In the case of a 1st/8th rank wrapping, the bit is pruned after the shift,
//! so no mask is required.

const A_FILE: u64 = 0x101010101010101;
const H_FILE: u64 = A_FILE << 7;

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
