use crate::types::{Bitboard, Color, Square};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

static mut BETWEEN: [[Bitboard; 64]; 64] = [[Bitboard(0); 64]; 64];

pub fn init() {
    fn between(a: Square, b: Square) -> &'static mut Bitboard {
        unsafe { &mut BETWEEN[a][b] }
    }

    for a in 0..64 {
        for b in 0..64 {
            let a = Square::new(a);
            let b = Square::new(b);

            if rook_attacks(a, Bitboard(0)).contains(b) {
                *between(a, b) = rook_attacks(a, b.to_bb()) & rook_attacks(b, a.to_bb());
            }

            if bishop_attacks(a, Bitboard(0)).contains(b) {
                *between(a, b) = bishop_attacks(a, b.to_bb()) & bishop_attacks(b, a.to_bb());
            }
        }
    }
}

pub fn between(a: Square, b: Square) -> Bitboard {
    unsafe { BETWEEN[a as usize][b as usize] }
}

pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    unsafe {
        match color {
            Color::White => Bitboard(*WHITE_PAWN_MAP.get_unchecked(square as usize)),
            Color::Black => Bitboard(*BLACK_PAWN_MAP.get_unchecked(square as usize)),
        }
    }
}

pub fn king_attacks(square: Square) -> Bitboard {
    unsafe { Bitboard(*KING_MAP.get_unchecked(square as usize)) }
}

pub fn knight_attacks(square: Square) -> Bitboard {
    unsafe { Bitboard(*KNIGHT_MAP.get_unchecked(square as usize)) }
}

pub fn rook_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    unsafe {
        let entry = ROOK_MAGICS.get_unchecked(square as usize);
        let index = magic_index(occupancies, entry);

        Bitboard(*ROOK_MAP.get_unchecked(index as usize))
    }
}

pub fn bishop_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    unsafe {
        let entry = BISHOP_MAGICS.get_unchecked(square as usize);
        let index = magic_index(occupancies, entry);

        Bitboard(*BISHOP_MAP.get_unchecked(index as usize))
    }
}

pub fn queen_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    rook_attacks(square, occupancies) | bishop_attacks(square, occupancies)
}

const fn magic_index(occupancies: Bitboard, entry: &MagicEntry) -> u32 {
    let mut hash = occupancies.0 & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as u32 + entry.offset
}
