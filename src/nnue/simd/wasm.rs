use std::{arch::wasm32::*, mem::size_of};

pub const F32_LANES: usize = size_of::<v128>() / size_of::<f32>();
pub const I32_LANES: usize = size_of::<v128>() / size_of::<i32>();
pub const I16_LANES: usize = size_of::<v128>() / size_of::<i16>();
pub const MUL_HI_SHIFT: i32 = 0;

pub fn add_i16(a: v128, b: v128) -> v128 {
    i16x8_add(a, b)
}

pub fn sub_i16(a: v128, b: v128) -> v128 {
    i16x8_sub(a, b)
}

pub unsafe fn zeroed() -> v128 {
    i32x4_splat(0)
}

pub unsafe fn splat_i16(a: i16) -> v128 {
    i16x8_splat(a)
}

pub unsafe fn clamp_i16(x: v128, min: v128, max: v128) -> v128 {
    i16x8_max(i16x8_min(x, max), min)
}

pub unsafe fn min_i16(a: v128, b: v128) -> v128 {
    i16x8_min(a, b)
}

pub unsafe fn shift_left_i16<const SHIFT: i32>(a: v128) -> v128 {
    i16x8_shl(a, SHIFT as u32)
}

pub unsafe fn mul_high_i16(a: v128, b: v128) -> v128 {
    let lo = i32x4_extmul_low_i16x8(a, b);
    let hi = i32x4_extmul_high_i16x8(a, b);
    i8x16_shuffle::<2, 3, 6, 7, 10, 11, 14, 15, 18, 19, 22, 23, 26, 27, 30, 31>(lo, hi)
}

pub unsafe fn convert_i8_i16(a: u64) -> v128 {
    i16x8_extend_low_i8x16(i64x2_splat(a as i64))
}

pub unsafe fn packus(a: v128, b: v128) -> v128 {
    u8x16_narrow_i16x8(a, b)
}

pub unsafe fn permute(a: v128) -> v128 {
    a
}

pub unsafe fn splat_i32(a: i32) -> v128 {
    i32x4_splat(a)
}

pub unsafe fn zero_f32() -> v128 {
    f32x4_splat(0.0)
}

pub unsafe fn splat_f32(a: f32) -> v128 {
    f32x4_splat(a)
}

pub unsafe fn mul_add_f32(a: v128, b: v128, c: v128) -> v128 {
    #[cfg(target_feature = "relaxed-simd")]
    return f32x4_relaxed_madd(a, b, c);
    #[cfg(not(target_feature = "relaxed-simd"))]
    return f32x4_add(f32x4_mul(a, b), c);
}

pub unsafe fn convert_to_f32(a: v128) -> v128 {
    f32x4_convert_i32x4(a)
}

pub unsafe fn clamp_f32(x: v128, min: v128, max: v128) -> v128 {
    #[cfg(target_feature = "relaxed-simd")]
    return f32x4_relaxed_max(f32x4_relaxed_min(x, max), min);
    #[cfg(not(target_feature = "relaxed-simd"))]
    return f32x4_max(f32x4_min(x, max), min);
}

pub unsafe fn dpbusd(i32s: v128, u8s: v128, i8s: v128) -> v128 {
    #[cfg(target_feature = "relaxed-simd")]
    return i32x4_relaxed_dot_i8x16_i7x16_add(i8s, u8s, i32s);
    #[cfg(not(target_feature = "relaxed-simd"))]
    {
        let dot_lo = i32x4_dot_i16x8(u16x8_extend_low_u8x16(u8s), i16x8_extend_low_i8x16(i8s));
        let dot_hi = i32x4_dot_i16x8(u16x8_extend_high_u8x16(u8s), i16x8_extend_high_i8x16(i8s));
        let even = i8x16_shuffle::<0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 24, 25, 26, 27>(dot_lo, dot_hi);
        let odd = i8x16_shuffle::<4, 5, 6, 7, 12, 13, 14, 15, 20, 21, 22, 23, 28, 29, 30, 31>(dot_lo, dot_hi);
        i32x4_add(i32s, i32x4_add(even, odd))
    }
}

pub unsafe fn double_dpbusd(i32s: v128, u8s1: v128, i8s1: v128, u8s2: v128, i8s2: v128) -> v128 {
    #[cfg(target_feature = "relaxed-simd")]
    return dpbusd(dpbusd(i32s, u8s1, i8s1), u8s2, i8s2);
    #[cfg(not(target_feature = "relaxed-simd"))]
    {
        let dot1_lo = i32x4_dot_i16x8(u16x8_extend_low_u8x16(u8s1), i16x8_extend_low_i8x16(i8s1));
        let dot2_lo = i32x4_dot_i16x8(u16x8_extend_low_u8x16(u8s2), i16x8_extend_low_i8x16(i8s2));
        let sum_lo = i32x4_add(dot1_lo, dot2_lo);
        let dot1_hi = i32x4_dot_i16x8(u16x8_extend_high_u8x16(u8s1), i16x8_extend_high_i8x16(i8s1));
        let dot2_hi = i32x4_dot_i16x8(u16x8_extend_high_u8x16(u8s2), i16x8_extend_high_i8x16(i8s2));
        let sum_hi = i32x4_add(dot1_hi, dot2_hi);
        let even = i8x16_shuffle::<0, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 24, 25, 26, 27>(sum_lo, sum_hi);
        let odd = i8x16_shuffle::<4, 5, 6, 7, 12, 13, 14, 15, 20, 21, 22, 23, 28, 29, 30, 31>(sum_lo, sum_hi);
        i32x4_add(i32s, i32x4_add(even, odd))
    }
}

pub unsafe fn horizontal_sum(x: [v128; 4]) -> f32 {
    let sum02 = f32x4_add(x[0], x[2]);
    let sum13 = f32x4_add(x[1], x[3]);
    let sum = f32x4_add(sum02, sum13);
    let rotated = i8x16_shuffle::<8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7>(sum, sum);
    let sum2 = f32x4_add(sum, rotated);
    f32x4_extract_lane::<0>(sum2) + f32x4_extract_lane::<1>(sum2)
}

pub unsafe fn nnz_bitmask(x: v128) -> u16 {
    i32x4_bitmask(i32x4_gt(x, i32x4_splat(0))) as u16
}
