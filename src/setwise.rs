use crate::types::{Bitboard, Color, File, Rank};

const A: Bitboard = Bitboard::file(File::A);
const B: Bitboard = Bitboard::file(File::B);
const G: Bitboard = Bitboard::file(File::G);
const H: Bitboard = Bitboard::file(File::H);
const R1: Bitboard = Bitboard::rank(Rank::R1);
const R2: Bitboard = Bitboard::rank(Rank::R2);
const R7: Bitboard = Bitboard::rank(Rank::R7);
const R8: Bitboard = Bitboard::rank(Rank::R8);

pub fn pawn_attacks_setwise(bb: Bitboard, color: Color) -> Bitboard {
    let (up_right, up_left) = match color {
        Color::White => (9, 7),
        Color::Black => (-7, -9),
    };

    let right_attacks = (bb & !Bitboard::file(File::H)).shift(up_right);
    let left_attacks = (bb & !Bitboard::file(File::A)).shift(up_left);

    right_attacks | left_attacks
}

#[cfg(not(target_feature = "avx2"))]
#[inline]
pub fn knight_attacks_setwise(bb: Bitboard) -> Bitboard {
    (bb & !(A | B | R8)).shift(6)
        | (bb & !(A | R7 | R8)).shift(15)
        | (bb & !(H | R7 | R8)).shift(17)
        | (bb & !(G | H | R8)).shift(10)
        | (bb & !(G | H | R1)).shift(-6)
        | (bb & !(H | R1 | R2)).shift(-15)
        | (bb & !(A | R1 | R2)).shift(-17)
        | (bb & !(A | B | R1)).shift(-10)
}

#[cfg(target_feature = "avx2")]
#[inline]
pub fn knight_attacks_setwise(bb: Bitboard) -> Bitboard {
    use core::arch::x86_64::*;

    unsafe {
        let mask_a = _mm256_set_epi64x(
            !(A | B | R8).0 as i64,
            !(A | R7 | R8).0 as i64,
            !(H | R7 | R8).0 as i64,
            !(G | H | R8).0 as i64,
        );
        let mask_b = _mm256_set_epi64x(
            !(G | H | R1).0 as i64,
            !(H | R1 | R2).0 as i64,
            !(A | R1 | R2).0 as i64,
            !(A | B | R1).0 as i64,
        );

        let bb = _mm256_set1_epi64x(bb.0 as i64);
        let a = _mm256_and_si256(bb, mask_a);
        let b = _mm256_and_si256(bb, mask_b);
        let a = _mm256_sllv_epi64(a, _mm256_set_epi64x(6, 15, 17, 10));
        let b = _mm256_srlv_epi64(b, _mm256_set_epi64x(6, 15, 17, 10));
        fold_to_bitboard(_mm256_or_si256(a, b))
    }
}

#[cfg(not(target_feature = "avx2"))]
#[inline]
pub fn bishop_attacks_setwise(bb: Bitboard, occupancies: Bitboard) -> Bitboard {
    use crate::lookup::bishop_attacks;

    let mut result = Bitboard(0);
    for square in bb {
        result |= bishop_attacks(square, occupancies);
    }
    result
}

#[cfg(target_feature = "avx2")]
#[inline]
pub fn bishop_attacks_setwise(bb: Bitboard, occupancies: Bitboard) -> Bitboard {
    use std::arch::x86_64::*;

    unsafe {
        let mask = _mm256_set_epi64x(!(R8 | H).0 as i64, !(R8 | A).0 as i64, !(R1 | H).0 as i64, !(R1 | A).0 as i64);

        let generate = _mm256_set1_epi64x(bb.0 as i64);
        let propagate = _mm256_and_si256(_mm256_set1_epi64x(!occupancies.0 as i64), mask);
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-9, -7, 7, 9>(generate)));
        let propagate = _mm256_and_si256(propagate, shiftv::<-9, -7, 7, 9>(propagate));
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-18, -14, 14, 18>(generate)));
        let propagate = _mm256_and_si256(propagate, shiftv::<-18, -14, 14, 18>(propagate));
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-36, -28, 28, 36>(generate)));
        let attacks = _mm256_and_si256(shiftv::<-9, -7, 7, 9>(generate), mask);

        fold_to_bitboard(attacks)
    }
}

#[cfg(not(target_feature = "avx2"))]
#[inline]
pub fn rook_attacks_setwise(bb: Bitboard, occupancies: Bitboard) -> Bitboard {
    use crate::lookup::rook_attacks;

    let mut result = Bitboard(0);
    for square in bb {
        result |= rook_attacks(square, occupancies);
    }
    result
}

#[cfg(target_feature = "avx2")]
#[inline]
pub fn rook_attacks_setwise(bb: Bitboard, occupancies: Bitboard) -> Bitboard {
    use std::arch::x86_64::*;

    unsafe {
        let mask = _mm256_set_epi64x(!R8.0 as i64, !H.0 as i64, !A.0 as i64, !R1.0 as i64);

        let generate = _mm256_set1_epi64x(bb.0 as i64);
        let propagate = _mm256_and_si256(_mm256_set1_epi64x(!occupancies.0 as i64), mask);
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-8, -1, 1, 8>(generate)));
        let propagate = _mm256_and_si256(propagate, shiftv::<-8, -1, 1, 8>(propagate));
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-16, -2, 2, 16>(generate)));
        let propagate = _mm256_and_si256(propagate, shiftv::<-16, -2, 2, 16>(propagate));
        let generate = _mm256_or_si256(generate, _mm256_and_si256(propagate, shiftv::<-32, -4, 4, 32>(generate)));
        let attacks = _mm256_and_si256(shiftv::<-8, -1, 1, 8>(generate), mask);

        fold_to_bitboard(attacks)
    }
}

#[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
#[inline]
unsafe fn shiftv<const A: i64, const B: i64, const C: i64, const D: i64>(
    vector: core::arch::x86_64::__m256i,
) -> core::arch::x86_64::__m256i {
    use core::arch::x86_64::*;

    _mm256_or_si256(
        _mm256_sllv_epi64(vector, _mm256_set_epi64x(A, B, C, D)),
        _mm256_srlv_epi64(vector, _mm256_set_epi64x(-A, -B, -C, -D)),
    )
}

#[cfg(target_feature = "avx512f")]
#[inline]
unsafe fn shiftv<const A: i64, const B: i64, const C: i64, const D: i64>(
    vector: core::arch::x86_64::__m256i,
) -> core::arch::x86_64::__m256i {
    use core::arch::x86_64::*;

    _mm256_rolv_epi64(vector, _mm256_set_epi64x(A, B, C, D))
}

#[cfg(target_feature = "avx2")]
#[inline]
unsafe fn fold_to_bitboard(vector: core::arch::x86_64::__m256i) -> Bitboard {
    use core::arch::x86_64::*;

    let vector = _mm_or_si128(_mm256_castsi256_si128(vector), _mm256_extracti128_si256::<1>(vector));
    let result = _mm_extract_epi64::<0>(vector) | _mm_extract_epi64::<1>(vector);
    Bitboard(result as u64)
}
