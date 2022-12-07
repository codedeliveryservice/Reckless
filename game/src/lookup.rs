use crate::core::{bitboard::Bitboard, square::Square};

mod attacks;
mod maps;

#[inline(always)]
pub fn king_attacks(square: Square) -> Bitboard {
    Bitboard(maps::KING_MAP[square.0 as usize])
}
