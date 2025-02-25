use crate::{
    lookup::{bishop_attacks, rook_attacks},
    types::{Bitboard, Square},
};

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

pub const fn between(a: Square, b: Square) -> Bitboard {
    unsafe { BETWEEN[a as usize][b as usize] }
}
