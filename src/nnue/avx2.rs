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

pub unsafe fn horizontal_sum(vec: __m256) -> f32 {
    let pairwise = _mm256_hadd_ps(vec, vec);
    let quad = _mm256_hadd_ps(pairwise, pairwise);

    let lo = _mm256_castps256_ps128(quad);
    let hi = _mm256_extractf128_ps::<1>(quad);

    _mm_cvtss_f32(_mm_add_ss(lo, hi))
}

pub unsafe fn nnz_bitmask(x: __m256i) -> u8 {
    let zero = _mm256_setzero_si256();
    let is_zero = _mm256_cmpeq_epi32(x, zero);
    let is_nonzero = _mm256_xor_si256(is_zero, _mm256_set1_epi32(-1));

    _mm256_movemask_ps(_mm256_castsi256_ps(is_nonzero)) as u8
}
