use crate::types::{Bitboard, Color, Piece, PieceType, Square, ZOBRIST};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

static mut BETWEEN: [[Bitboard; 64]; 64] = [[Bitboard(0); 64]; 64];

static mut CUCKOO: [u64; 0x2000] = [0; 0x2000];
static mut A: [Square; 0x2000] = [Square::None; 0x2000];
static mut B: [Square; 0x2000] = [Square::None; 0x2000];

const LEN: usize = 11;
static mut LMR_INTERACTIONS: [i32; 1 << LEN] = [0; 1 << LEN];

pub fn init() {
    unsafe {
        init_between();
        init_cuckoo();
        init_lmr_interactions();
    }
}

unsafe fn init_between() {
    for a in 0..64 {
        for b in 0..64 {
            let a = Square::new(a);
            let b = Square::new(b);

            if rook_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = rook_attacks(a, b.to_bb()) & rook_attacks(b, a.to_bb());
            }

            if bishop_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = bishop_attacks(a, b.to_bb()) & bishop_attacks(b, a.to_bb());
            }
        }
    }
}

unsafe fn init_cuckoo() {
    fn is_reversible_move(piece: Piece, a: Square, b: Square) -> bool {
        match piece.piece_type() {
            PieceType::Knight => knight_attacks(a).contains(b),
            PieceType::Bishop => bishop_attacks(a, Bitboard(0)).contains(b),
            PieceType::Rook => rook_attacks(a, Bitboard(0)).contains(b),
            PieceType::Queen => queen_attacks(a, Bitboard(0)).contains(b),
            PieceType::King => king_attacks(a).contains(b),
            _ => unreachable!(),
        }
    }

    for index in 2..12 {
        let piece = Piece::from_index(index);

        debug_assert!(piece.piece_type() != PieceType::Pawn);

        for a in 0..64 {
            for b in (a + 1)..64 {
                let mut a = Square::new(a);
                let mut b = Square::new(b);

                if !is_reversible_move(piece, a, b) {
                    continue;
                }

                let mut mv = ZOBRIST.pieces[piece][a] ^ ZOBRIST.pieces[piece][b] ^ ZOBRIST.side;
                let mut i = h1(mv);

                loop {
                    std::mem::swap(&mut CUCKOO[i], &mut mv);
                    std::mem::swap(&mut A[i], &mut a);
                    std::mem::swap(&mut B[i], &mut b);

                    if a == Square::None && b == Square::None {
                        break;
                    }

                    i = if i == h1(mv) { h2(mv) } else { h1(mv) };
                }
            }
        }
    }
}

pub const fn h1(h: u64) -> usize {
    ((h >> 32) & 0x1fff) as usize
}

pub const fn h2(h: u64) -> usize {
    ((h >> 48) & 0x1fff) as usize
}

pub fn cuckoo(index: usize) -> u64 {
    unsafe { CUCKOO[index] }
}

pub fn cuckoo_a(index: usize) -> Square {
    unsafe { A[index] }
}

pub fn cuckoo_b(index: usize) -> Square {
    unsafe { B[index] }
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

unsafe fn init_lmr_interactions() {
    const SINGLE_VALUES_LEN: usize = LEN;
    const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;

    const SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-744, -608, 1717, -92, 121, -94, 236, -28, 1417, -767, -915];
    const DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
        41, -90, 40, 24, -19, 199, -489, -169, -54, 57, -727, -546, 48, -971, 57, 18, 11, -25, -156, 3, -139, -3, -25,
        -70, -33, 9, 132, 109, -44, -50, -6, 37, 125, 83, 92, 886, 246, -104, 62, -30, -109, -3, -174, -87, 51, 118,
        15, 52, -60, -117, 44, -100, 26, 66, 34,
    ];

    for mask in 0..(1 << LEN) {
        let mut s = 0i32;
        // singles
        for i in 0..LEN {
            if (mask >> i) & 1 != 0 {
                s += SINGLE_VALUES[i];
            }
        }
        // doubles
        let mut idx = 0usize;
        for i in 0..LEN {
            if (mask >> i) & 1 == 0 {
                idx += LEN - i - 1;
                continue;
            }
            for j in (i + 1)..LEN {
                if (mask >> j) & 1 != 0 {
                    s += DOUBLE_VALUES[idx];
                }
                idx += 1;
            }
        }
        LMR_INTERACTIONS[mask] = s;
    }
}

pub fn lmr_rules_reduction(features: &[bool]) -> i32 {
    debug_assert!(features.len() >= LEN);
    let mut mask = 0usize;
    for i in 0..LEN {
        mask |= (features[i] as usize) << i;
    }
    unsafe { LMR_INTERACTIONS[mask] }
}
