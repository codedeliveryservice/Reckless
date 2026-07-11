use std::{arch::x86_64::*, mem::size_of};

pub const I32_LANES: usize = size_of::<__m256i>() / size_of::<i32>();
pub const I16_LANES: usize = size_of::<__m256i>() / size_of::<i16>();
pub const MUL_HI_SHIFT: i32 = 0;

pub fn add_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_add_epi16(a, b) }
}

pub fn sub_i16(a: __m256i, b: __m256i) -> __m256i {
    unsafe { _mm256_sub_epi16(a, b) }
}

pub unsafe fn zeroed() -> __m256i {
    _mm256_setzero_si256()
}

pub unsafe fn splat_i16(a: i16) -> __m256i {
    _mm256_set1_epi16(a)
}

pub unsafe fn clamp_i16(x: __m256i, min: __m256i, max: __m256i) -> __m256i {
    _mm256_max_epi16(_mm256_min_epi16(x, max), min)
}

pub unsafe fn min_i16(a: __m256i, b: __m256i) -> __m256i {
    _mm256_min_epi16(a, b)
}

pub unsafe fn shift_left_i16<const SHIFT: i32>(a: __m256i) -> __m256i {
    _mm256_slli_epi16::<SHIFT>(a)
}

pub unsafe fn mul_high_i16(a: __m256i, b: __m256i) -> __m256i {
    _mm256_mulhi_epi16(a, b)
}

pub unsafe fn convert_i8_i16(a: __m128i) -> __m256i {
    _mm256_cvtepi8_epi16(a)
}

pub unsafe fn packus(a: __m256i, b: __m256i) -> __m256i {
    _mm256_packus_epi16(a, b)
}

pub unsafe fn permute(a: __m256i) -> __m256i {
    _mm256_permute4x64_epi64::<0b11_01_10_00>(a)
}

pub unsafe fn splat_i32(a: i32) -> __m256i {
    _mm256_set1_epi32(a)
}

pub unsafe fn madd_i16(a: __m256i, b: __m256i) -> __m256i {
    _mm256_madd_epi16(a, b)
}

pub unsafe fn add_i32(a: __m256i, b: __m256i) -> __m256i {
    _mm256_add_epi32(a, b)
}

pub unsafe fn shift_right_i32<const SHIFT: i32>(a: __m256i) -> __m256i {
    _mm256_srai_epi32::<SHIFT>(a)
}

pub unsafe fn clamp_i32(x: __m256i, min: __m256i, max: __m256i) -> __m256i {
    _mm256_max_epi32(_mm256_min_epi32(x, max), min)
}

pub unsafe fn pack_i32(a: __m256i, b: __m256i) -> __m256i {
    permute(_mm256_packs_epi32(a, b))
}

pub unsafe fn horizontal_sum_i32(x: __m256i) -> i32 {
    let sum128 = _mm_add_epi32(_mm256_castsi256_si128(x), _mm256_extracti128_si256::<1>(x));
    let sum64 = _mm_add_epi32(sum128, _mm_unpackhi_epi64(sum128, sum128));
    let sum32 = _mm_add_epi32(sum64, _mm_shuffle_epi32::<1>(sum64));
    _mm_cvtsi128_si32(sum32)
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

pub unsafe fn nnz_bitmask(x: __m256i) -> u16 {
    let greater_than_zero = _mm256_cmpgt_epi32(x, _mm256_setzero_si256());
    _mm256_movemask_ps(_mm256_castsi256_ps(greater_than_zero)) as u16
}
