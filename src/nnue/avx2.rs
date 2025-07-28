use std::{arch::x86_64::*, mem::size_of};

pub const F32_LANES: usize = size_of::<__m256>() / size_of::<f32>();
pub const I16_LANES: usize = size_of::<__m256i>() / size_of::<i16>();

pub fn add_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi16(a, b) }
}

pub fn sub_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_sub_epi16(a, b) }
}

pub unsafe fn dpbusd(i32s: __m256i, u8s: __m256i, i8s: __m256i) -> __m256i {
    let pairwise = _mm256_maddubs_epi16(u8s, i8s);
    let widened = _mm256_madd_epi16(pairwise, _mm256_set1_epi16(1));
    _mm256_add_epi32(i32s, widened)
}

pub unsafe fn double_dpbusd(i32s: __m256i, u8s1: __m256i, i8s1: __m256i, u8s2: __m256i, i8s2: __m256i) -> __m256i {
    let pairwise1 = _mm256_maddubs_epi16(u8s1, i8s1);
    let pairwise2 = _mm256_maddubs_epi16(u8s2, i8s2);
    let widened = _mm256_madd_epi16(_mm256_add_epi16(pairwise1, pairwise2), _mm256_set1_epi16(1));
    _mm256_add_epi32(i32s, widened)
}

pub unsafe fn horizontal_sum(vec: __m256) -> f32 {
    let hi128 = _mm256_extractf128_ps::<1>(vec);
    let lo128 = _mm256_castps256_ps128(vec);
    let sum128 = _mm_add_ps(lo128, hi128);

    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);

    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x1);
    let sum32 = _mm_add_ss(sum64, hi32);

    _mm_cvtss_f32(sum32)
}

pub unsafe fn nnz_bitmask(x: __m256i) -> u8 {
    let greater_than_zero = _mm256_cmpgt_epi32(x, _mm256_setzero_si256());
    _mm256_movemask_ps(_mm256_castsi256_ps(greater_than_zero)) as u8
}
