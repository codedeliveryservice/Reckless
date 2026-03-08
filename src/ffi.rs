use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, Color, Square},
};

#[unsafe(no_mangle)]
pub extern "C" fn reckless_popcount(bitboard: u64) -> u32 {
    bitboard.count_ones()
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_lsb(bitboard: u64) -> u32 {
    bitboard.trailing_zeros()
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_poplsb(bitboard: *mut u64) -> u64 {
    unsafe {
        let value = *bitboard;
        let lsb = value.trailing_zeros();
        *bitboard = value & (value - 1);
        lsb as u64
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_pawn_attacks(square: u32, color: bool) -> u64 {
    pawn_attacks(Square::new(square as u8), if color { Color::White } else { Color::Black }).0
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_knight_attacks(square: u32) -> u64 {
    knight_attacks(Square::new(square as u8)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_bishop_attacks(square: u32, occupancies: u64) -> u64 {
    bishop_attacks(Square::new(square as u8), Bitboard(occupancies)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_rook_attacks(square: u32, occupancies: u64) -> u64 {
    rook_attacks(Square::new(square as u8), Bitboard(occupancies)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_queen_attacks(square: u32, occupancies: u64) -> u64 {
    queen_attacks(Square::new(square as u8), Bitboard(occupancies)).0
}

#[unsafe(no_mangle)]
pub extern "C" fn reckless_king_attacks(square: u32) -> u64 {
    king_attacks(Square::new(square as u8)).0
}
