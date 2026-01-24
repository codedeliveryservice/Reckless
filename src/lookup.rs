use crate::types::{Bitboard, Color, File, Piece, PieceType, Square, ZOBRIST};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

static mut BETWEEN: [[Bitboard; 64]; 64] = [[Bitboard(0); 64]; 64];
static mut RAY_PASS: [[Bitboard; 64]; 64] = [[Bitboard(0); 64]; 64];

static mut CUCKOO: [u64; 0x2000] = [0; 0x2000];
static mut A: [Square; 0x2000] = [Square::None; 0x2000];
static mut B: [Square; 0x2000] = [Square::None; 0x2000];

pub fn initialize() {
    unsafe {
        init_luts();
        init_cuckoo();
    }
}

unsafe fn init_luts() {
    for a in 0..64 {
        for b in 0..64 {
            let a = Square::new(a);
            let b = Square::new(b);

            if rook_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = rook_attacks(a, b.to_bb()) & rook_attacks(b, a.to_bb());
                RAY_PASS[a][b] = rook_attacks(a, Bitboard(0)) & rook_attacks(b, a.to_bb());
            }

            if bishop_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = bishop_attacks(a, b.to_bb()) & bishop_attacks(b, a.to_bb());
                RAY_PASS[a][b] = bishop_attacks(a, Bitboard(0)) & bishop_attacks(b, a.to_bb());
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

#[allow(dead_code)]
pub fn ray_pass(a: Square, b: Square) -> Bitboard {
    unsafe { RAY_PASS[a as usize][b as usize] }
}

pub fn attacks(piece: Piece, square: Square, occupancies: Bitboard) -> Bitboard {
    match piece.piece_type() {
        PieceType::Pawn => pawn_attacks(square, piece.piece_color()),
        PieceType::Knight => knight_attacks(square),
        PieceType::Bishop => bishop_attacks(square, occupancies),
        PieceType::Rook => rook_attacks(square, occupancies),
        PieceType::Queen => queen_attacks(square, occupancies),
        PieceType::King => king_attacks(square),
        PieceType::None => Bitboard(0),
    }
}

pub fn pawn_attacks_setwise(bb: Bitboard, color: Color) -> Bitboard {
    let (up_right, up_left) = match color {
        Color::White => (9, 7),
        Color::Black => (-7, -9),
    };

    let right_attacks = (bb & !Bitboard::file(File::H)).shift(up_right);
    let left_attacks = (bb & !Bitboard::file(File::A)).shift(up_left);

    right_attacks | left_attacks
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

#[cfg(target_feature = "avx512f")]
pub fn slider_attacks_setwise(bishops: Bitboard, rooks: Bitboard, queens: Bitboard, occupancies: Bitboard) -> Bitboard {
    use crate::types::{File, Rank};
    use std::arch::x86_64::*;
    unsafe {
        let attackers = _mm512_mask_blend_epi64(
            0x0F,
            _mm512_set1_epi64((rooks | queens).0 as i64),
            _mm512_set1_epi64((bishops | queens).0 as i64),
        );

        let rotates1 = _mm512_set_epi64(-8, -1, 1, 8, -9, -7, 7, 9);
        let rotates2 = _mm512_add_epi64(rotates1, rotates1);
        let rotates4 = _mm512_add_epi64(rotates2, rotates2);

        let masks = _mm512_set_epi64(
            !Bitboard::rank(Rank::R8).0 as i64,
            !Bitboard::file(File::H).0 as i64,
            !Bitboard::file(File::A).0 as i64,
            !Bitboard::rank(Rank::R1).0 as i64,
            (!Bitboard::rank(Rank::R8) & !Bitboard::file(File::H)).0 as i64,
            (!Bitboard::rank(Rank::R8) & !Bitboard::file(File::A)).0 as i64,
            (!Bitboard::rank(Rank::R1) & !Bitboard::file(File::H)).0 as i64,
            (!Bitboard::rank(Rank::R1) & !Bitboard::file(File::A)).0 as i64,
        );

        // Koggle-Stone algorithm
        let generate = attackers;
        let propagate = _mm512_and_si512(_mm512_set1_epi64(!occupancies.0 as i64), masks);
        let generate = _mm512_or_si512(generate, _mm512_and_si512(propagate, _mm512_rolv_epi64(generate, rotates1)));
        let propagate = _mm512_and_si512(propagate, _mm512_rolv_epi64(propagate, rotates1));
        let generate = _mm512_or_si512(generate, _mm512_and_si512(propagate, _mm512_rolv_epi64(generate, rotates2)));
        let propagate = _mm512_and_si512(propagate, _mm512_rolv_epi64(propagate, rotates2));
        let generate = _mm512_or_si512(generate, _mm512_and_si512(propagate, _mm512_rolv_epi64(generate, rotates4)));
        let attacks = _mm512_and_si512(_mm512_rolv_epi64(generate, rotates1), masks);

        // Fold attacks
        match () {
            #[cfg(all(target_feature = "avx512bw", target_feature = "avx512vbmi", target_feature = "gfni"))]
            _ => {
                let attacks = _mm512_gf2p8affine_epi64_epi8(
                    _mm512_set1_epi64(0x8040201008040201u64 as i64),
                    _mm512_permutexvar_epi8(
                        _mm512_set_epi8(
                            7, 15, 23, 31, 39, 47, 55, 63, 6, 14, 22, 30, 38, 46, 54, 62, 5, 13, 21, 29, 37, 45, 53,
                            61, 4, 12, 20, 28, 36, 44, 52, 60, 3, 11, 19, 27, 35, 43, 51, 59, 2, 10, 18, 26, 34, 42,
                            50, 58, 1, 9, 17, 25, 33, 41, 49, 57, 0, 8, 16, 24, 32, 40, 48, 56,
                        ),
                        attacks,
                    ),
                    0,
                );
                Bitboard(_mm512_test_epi8_mask(attacks, attacks))
            }
            #[allow(unreachable_patterns)]
            _ => {
                let attacks = _mm256_or_si256(_mm512_castsi512_si256(attacks), _mm512_extracti64x4_epi64::<1>(attacks));
                let attacks = _mm_or_si128(_mm256_castsi256_si128(attacks), _mm256_extracti128_si256::<1>(attacks));
                let attacks = _mm_extract_epi64::<0>(attacks) | _mm_extract_epi64::<1>(attacks);
                Bitboard(attacks as u64)
            }
        }
    }
}

const fn magic_index(occupancies: Bitboard, entry: &MagicEntry) -> u32 {
    let mut hash = occupancies.0 & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as u32 + entry.offset
}
