use crate::attacks;

pub fn king_map() -> [u64; 64] {
    let mut map = [0; 64];

    let mut square = 0;
    while square < 64 {
        map[square as usize] = attacks::king_attacks(square as u8);
        square += 1;
    }

    map
}
