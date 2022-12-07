//! Contains constant arrays of move maps.

use super::attacks;

pub const KING_MAP: [u64; 64] = {
    let mut map = [0; 64];

    let mut square = 0;
    while square < 64 {
        map[square as usize] = attacks::king_attacks(square as u8);
        square += 1;
    }

    map
};
