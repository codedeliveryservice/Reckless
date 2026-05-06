use crate::{attacks::*, magics::*};

pub fn generate_king_map() -> [u64; 64] {
    generate_map(king_attacks)
}

pub fn generate_knight_map() -> [u64; 64] {
    generate_map(knight_attacks)
}

fn generate_map<F: Fn(u8) -> u64>(f: F) -> [u64; 64] {
    let mut map = [0; 64];
    for square in 0..64 {
        map[square as usize] = f(square as u8);
    }

    map
}

pub fn generate_rays_map() -> [[u64; 64]; 64] {
    [
        generate_map(|square| directional_ray(square,  0)),
        generate_map(|square| directional_ray(square,  1)),
        generate_map(|square| directional_ray(square,  2)),
        generate_map(|square| directional_ray(square,  3)),
        generate_map(|square| directional_ray(square,  4)),
        generate_map(|square| directional_ray(square,  5)),
        generate_map(|square| directional_ray(square,  6)),
        generate_map(|square| directional_ray(square,  7)),
        generate_map(|square| directional_ray(square,  8)),
        generate_map(|square| directional_ray(square,  9)),
        generate_map(|square| directional_ray(square, 10)),
        generate_map(|square| directional_ray(square, 11)),
        generate_map(|square| directional_ray(square, 12)),
        generate_map(|square| directional_ray(square, 13)),
        generate_map(|square| directional_ray(square, 14)),
        generate_map(|square| directional_ray(square, 15)),
        generate_map(|square| directional_ray(square, 16)),
        generate_map(|square| directional_ray(square, 17)),
        generate_map(|square| directional_ray(square, 18)),
        generate_map(|square| directional_ray(square, 19)),
        generate_map(|square| directional_ray(square, 20)),
        generate_map(|square| directional_ray(square, 21)),
        generate_map(|square| directional_ray(square, 22)),
        generate_map(|square| directional_ray(square, 23)),
        generate_map(|square| directional_ray(square, 24)),
        generate_map(|square| directional_ray(square, 25)),
        generate_map(|square| directional_ray(square, 26)),
        generate_map(|square| directional_ray(square, 27)),
        generate_map(|square| directional_ray(square, 28)),
        generate_map(|square| directional_ray(square, 29)),
        generate_map(|square| directional_ray(square, 30)),
        generate_map(|square| directional_ray(square, 31)),
        generate_map(|square| directional_ray(square, 32)),
        generate_map(|square| directional_ray(square, 33)),
        generate_map(|square| directional_ray(square, 34)),
        generate_map(|square| directional_ray(square, 35)),
        generate_map(|square| directional_ray(square, 36)),
        generate_map(|square| directional_ray(square, 37)),
        generate_map(|square| directional_ray(square, 38)),
        generate_map(|square| directional_ray(square, 39)),
        generate_map(|square| directional_ray(square, 40)),
        generate_map(|square| directional_ray(square, 41)),
        generate_map(|square| directional_ray(square, 42)),
        generate_map(|square| directional_ray(square, 43)),
        generate_map(|square| directional_ray(square, 44)),
        generate_map(|square| directional_ray(square, 45)),
        generate_map(|square| directional_ray(square, 46)),
        generate_map(|square| directional_ray(square, 47)),
        generate_map(|square| directional_ray(square, 48)),
        generate_map(|square| directional_ray(square, 49)),
        generate_map(|square| directional_ray(square, 50)),
        generate_map(|square| directional_ray(square, 51)),
        generate_map(|square| directional_ray(square, 52)),
        generate_map(|square| directional_ray(square, 53)),
        generate_map(|square| directional_ray(square, 54)),
        generate_map(|square| directional_ray(square, 55)),
        generate_map(|square| directional_ray(square, 56)),
        generate_map(|square| directional_ray(square, 57)),
        generate_map(|square| directional_ray(square, 58)),
        generate_map(|square| directional_ray(square, 59)),
        generate_map(|square| directional_ray(square, 60)),
        generate_map(|square| directional_ray(square, 61)),
        generate_map(|square| directional_ray(square, 62)),
        generate_map(|square| directional_ray(square, 63)),
    ]
}

pub fn generate_pawn_map() -> [[u64; 64]; 2] {
    [
        generate_map(|square| pawn_attacks(square, Color::White)),
        generate_map(|square| pawn_attacks(square, Color::Black)),
    ]
}

pub fn generate_diagonal_tables() -> [[u64; 64]; 2] {
    [
        generate_map(|square| sliding_attacks(square, 0, &[9, -9])),
        generate_map(|square| sliding_attacks(square, 0, &[7, -7])),
    ]
}

pub fn generate_rook_map() -> Vec<u64> {
    generate_sliding_map(ROOK_MAP_SIZE, &ROOK_MAGICS, &[8, -8, 1, -1])
}

pub fn generate_bishop_map() -> Vec<u64> {
    generate_sliding_map(BISHOP_MAP_SIZE, &BISHOP_MAGICS, &[9, 7, -7, -9])
}

fn generate_sliding_map(size: usize, magics: &[MagicEntry], directions: &[i8]) -> Vec<u64> {
    let mut map = vec![0; size];

    for square in 0..64 {
        let entry = &magics[square as usize];

        let mut occupancies = 0u64;
        for _ in 0..get_permutation_count(entry.mask) {
            let hash = magic_index(occupancies, entry);
            map[hash] = sliding_attacks(square, occupancies, directions);

            occupancies = occupancies.wrapping_sub(entry.mask) & entry.mask;
        }
    }

    map
}

const fn get_permutation_count(mask: u64) -> u64 {
    1 << mask.count_ones()
}

const fn magic_index(occupancies: u64, entry: &MagicEntry) -> usize {
    let mut hash = occupancies & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as usize + entry.offset
}
