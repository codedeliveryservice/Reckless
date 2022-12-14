use crate::{attacks::*, magics::*};

pub fn generate_king_map() -> [u64; 64] {
    generate_map(king_attacks)
}

pub fn generate_knight_map() -> [u64; 64] {
    generate_map(knight_attacks)
}

fn generate_map<T: Fn(u8) -> u64>(gen: T) -> [u64; 64] {
    let mut map = [0; 64];
    for square in 0..64 {
        map[square as usize] = gen(square as u8);
    }

    map
}

pub fn generate_rook_map() -> Vec<u64> {
    generate_sliding_map(
        ROOK_MAP_SIZE,
        &ROOK_MAGICS,
        &[(1, 0), (-1, 0), (0, 1), (0, -1)],
    )
}

pub fn generate_bishop_map() -> Vec<u64> {
    generate_sliding_map(
        BISHOP_MAP_SIZE,
        &BISHOP_MAGICS,
        &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
    )
}

fn generate_sliding_map(size: usize, magics: &[MagicEntry], directions: &[(i8, i8)]) -> Vec<u64> {
    let mut map = vec![0; size];

    for square in 0..64 {
        let entry = &magics[square as usize];

        let mut occupancies = 0u64;
        for _ in 0..get_permutation_count(entry.mask) {
            let hash = magic_index(occupancies, entry) as usize;
            map[hash] = sliding_attacks(square, occupancies, directions);

            occupancies = occupancies.wrapping_sub(entry.mask) & entry.mask;
        }
    }

    map
}

fn get_permutation_count(mask: u64) -> u64 {
    1 << mask.count_ones()
}

fn magic_index(occupancies: u64, entry: &MagicEntry) -> u32 {
    let mut hash = occupancies & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as u32 + entry.offset
}
