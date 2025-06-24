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
