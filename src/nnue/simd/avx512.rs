use std::{arch::x86_64::*, mem::size_of};

use crate::nnue::{Aligned, SparseEntry, L1_SIZE};

pub const F32_LANES: usize = size_of::<__m512>() / size_of::<f32>();
pub const I16_LANES: usize = size_of::<__m512i>() / size_of::<i16>();

pub fn add_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_add_epi16(a, b) }
}

pub fn sub_i16(a: __m512i, b: __m512i) -> __m512i {
    unsafe { _mm512_sub_epi16(a, b) }
}

pub unsafe fn zeroed() -> __m512i {
    _mm512_setzero_si512()
}

pub unsafe fn splat_i16(a: i16) -> __m512i {
    _mm512_set1_epi16(a)
}

pub unsafe fn clamp_i16(x: __m512i, min: __m512i, max: __m512i) -> __m512i {
    _mm512_max_epi16(_mm512_min_epi16(x, max), min)
}

pub unsafe fn min_i16(a: __m512i, b: __m512i) -> __m512i {
    _mm512_min_epi16(a, b)
}

pub unsafe fn shift_left_i16<const SHIFT: u32>(a: __m512i) -> __m512i {
    _mm512_slli_epi16::<SHIFT>(a)
}

pub unsafe fn mul_high_i16(a: __m512i, b: __m512i) -> __m512i {
    _mm512_mulhi_epi16(a, b)
}

pub unsafe fn packus(a: __m512i, b: __m512i) -> __m512i {
    _mm512_packus_epi16(a, b)
}

pub unsafe fn permute(a: __m512i) -> __m512i {
    _mm512_permutexvar_epi64(_mm512_setr_epi64(0, 2, 4, 6, 1, 3, 5, 7), a)
}

pub unsafe fn splat_i32(a: i32) -> __m512i {
    _mm512_set1_epi32(a)
}

pub unsafe fn zero_f32() -> __m512 {
    _mm512_setzero_ps()
}

pub unsafe fn splat_f32(a: f32) -> __m512 {
    _mm512_set1_ps(a)
}

pub unsafe fn mul_add_f32(a: __m512, b: __m512, c: __m512) -> __m512 {
    _mm512_fmadd_ps(a, b, c)
}

pub unsafe fn convert_to_f32(a: __m512i) -> __m512 {
    _mm512_cvtepi32_ps(a)
}

pub unsafe fn clamp_f32(x: __m512, min: __m512, max: __m512) -> __m512 {
    _mm512_max_ps(_mm512_min_ps(x, max), min)
}

pub unsafe fn dpbusd(i32s: __m512i, u8s: __m512i, i8s: __m512i) -> __m512i {
    let pairwise = _mm512_maddubs_epi16(u8s, i8s);
    let widened = _mm512_madd_epi16(pairwise, _mm512_set1_epi16(1));
    _mm512_add_epi32(i32s, widened)
}

pub unsafe fn double_dpbusd(i32s: __m512i, u8s1: __m512i, i8s1: __m512i, u8s2: __m512i, i8s2: __m512i) -> __m512i {
    let pairwise1 = _mm512_maddubs_epi16(u8s1, i8s1);
    let pairwise2 = _mm512_maddubs_epi16(u8s2, i8s2);
    let widened = _mm512_madd_epi16(_mm512_add_epi16(pairwise1, pairwise2), _mm512_set1_epi16(1));
    _mm512_add_epi32(i32s, widened)
}

pub unsafe fn horizontal_sum(x: [__m512; 1]) -> f32 {
    _mm512_reduce_add_ps(x[0])
}

// #[cfg(target_feature = "avx512vbmi2")]
pub unsafe fn find_nnz(ft_out: &[u8; L1_SIZE], _: &[SparseEntry]) -> (Aligned<[u16; L1_SIZE / 4]>, usize) {
    let mut indexes = Aligned::new([0; L1_SIZE / 4]);
    let mut count = 0;

    let increment = _mm512_set1_epi16(32);
    let mut base = _mm512_set_epi16(
        31, 30, 29, 28, 15, 14, 13, 12, 27, 26, 25, 24, 11, 10, 9, 8, 23, 22, 21, 20, 7, 6, 5, 4, 19, 18, 17, 16, 3, 2,
        1, 0,
    );

    for i in (0..L1_SIZE).step_by(128) {
        let input1 = *ft_out.as_ptr().add(i).cast();
        let input2 = *ft_out.as_ptr().add(i + 64).cast();
        let input = _mm512_packus_epi32(input1, input2);

        let mask = _mm512_test_epi16_mask(input, input);
        let masked = _mm512_maskz_compress_epi16(mask, base);

        let store = indexes.as_mut_ptr().add(count).cast();
        _mm512_store_si512(store, masked);

        count += mask.count_ones() as usize;
        base = _mm512_add_epi16(base, increment);
    }

    (indexes, count)
}

// #[cfg(not(target_feature = "avx512vbmi2"))]
// pub unsafe fn find_nnz(ft_out: &[u8; L1_SIZE], _: &[SparseEntry]) -> (Aligned<[u16; L1_SIZE / 4]>, usize) {
//     let mut indexes = Aligned::new([0; L1_SIZE / 4]);
//     let mut count = 0;

//     let increment = _mm512_set1_epi32(16);
//     let mut base = _mm512_set_epi32(15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0);

//     for i in (0..L1_SIZE).step_by(64) {
//         let input = *ft_out.as_ptr().add(i).cast();

//         let mask = _mm512_test_epi32_mask(input, input);
//         let masked = _mm512_maskz_compress_epi32(mask, base);

//         let store = indexes.as_mut_ptr().add(count).cast();
//         _mm512_mask_cvtepi32_storeu_epi16(store, 0xFFFF, masked);

//         count += mask.count_ones() as usize;
//         base = _mm512_add_epi32(base, increment);
//     }

//     (indexes, count)
// }
