use crate::core::{bitboard::Bitboard, square::Square};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

#[inline(always)]
pub fn king_attacks(square: Square) -> Bitboard {
    Bitboard(KING_MAP[square.0 as usize])
}
