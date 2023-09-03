use crate::types::{Bitboard, Color, Square};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    Bitboard(match color {
        Color::White => WHITE_PAWN_MAP[square],
        Color::Black => BLACK_PAWN_MAP[square],
    })
}

pub fn king_attacks(square: Square) -> Bitboard {
    Bitboard(KING_MAP[square])
}

pub fn knight_attacks(square: Square) -> Bitboard {
    Bitboard(KNIGHT_MAP[square])
}

pub fn rook_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    let entry = &ROOK_MAGICS[square];
    let index = magic_index(occupancies, entry);

    Bitboard(ROOK_MAP[index as usize])
}

pub fn bishop_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    let entry = &BISHOP_MAGICS[square];
    let index = magic_index(occupancies, entry);

    Bitboard(BISHOP_MAP[index as usize])
}

pub fn queen_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    rook_attacks(square, occupancies) | bishop_attacks(square, occupancies)
}

fn magic_index(occupancies: Bitboard, entry: &MagicEntry) -> u32 {
    let mut hash = occupancies.0 & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as u32 + entry.offset
}
