use std::{arch::aarch64::*, mem::size_of};

pub const F32_LANES: usize = size_of::<float32x4_t>() / size_of::<f32>();
pub const I16_LANES: usize = size_of::<int16x8_t>() / size_of::<i16>();

pub fn add_i16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
    unsafe { vaddq_s16(a, b) }
}

pub fn sub_i16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
    unsafe { vsubq_s16(a, b) }
}

pub unsafe fn zeroed() -> int32x4_t {
    vdupq_n_s32(0)
}

pub unsafe fn splat_i16(a: i16) -> int16x8_t {
    vdupq_n_s16(a)
}

pub unsafe fn clamp_i16(x: int16x8_t, min: int16x8_t, max: int16x8_t) -> int16x8_t {
    vmaxq_s16(vminq_s16(x, max), min)
}

pub unsafe fn min_i16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
    vminq_s16(a, b)
}

pub unsafe fn shift_left_i16<const SHIFT: i32>(a: int16x8_t) -> int16x8_t {
    vshlq_n_s16::<SHIFT>(a)
}

pub unsafe fn mul_high_i16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
    vmulq_s16(a, b)
}

pub unsafe fn mul_high_i16(a: int16x8_t, b: int16x8_t) -> int16x8_t {
    let low = vmull_s16(vget_low_s16(a), vget_low_s16(b));
    let high = vmull_s16(vget_high_s16(a), vget_high_s16(b));

    let low_hi = vshrn_n_s32::<16>(low);
    let high_hi = vshrn_n_s32::<16>(high);

    vcombine_s16(low_hi, high_hi)
}

pub unsafe fn convert_i8_i16(a: int8x8_t) -> int16x8_t {
    vmovl_s8(a)
}

pub unsafe fn packus(a: int16x8_t, b: int16x8_t) -> int8x16_t {
    let a_u8 = vqmovun_s16(a);
    let b_u8 = vqmovun_s16(b);
    vreinterpretq_s8_u8(vcombine_u8(a_u8, b_u8))
}

pub unsafe fn permute(a: int8x16_t) -> int8x16_t {
    a
}

pub unsafe fn splat_i32(a: i32) -> int32x4_t {
    vdupq_n_s32(a)
}

pub unsafe fn zero_f32() -> float32x4_t {
    vdupq_n_f32(0.0)
}

pub unsafe fn splat_f32(a: f32) -> float32x4_t {
    vdupq_n_f32(a)
}

pub unsafe fn mul_add_f32(a: float32x4_t, b: float32x4_t, c: float32x4_t) -> float32x4_t {
    vfmaq_f32(c, a, b)
}

pub unsafe fn convert_to_f32(a: int32x4_t) -> float32x4_t {
    vcvtq_f32_s32(a)
}

pub unsafe fn clamp_f32(x: float32x4_t, min: float32x4_t, max: float32x4_t) -> float32x4_t {
    vmaxq_f32(vminq_f32(x, max), min)
}

unsafe fn dot_bytes(u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    let u8s = vreinterpretq_u8_s32(u8s);

    let products_low = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_low_u8(u8s))), vmovl_s8(vget_low_s8(i8s)));
    let products_high = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_high_u8(u8s))), vmovl_s8(vget_high_s8(i8s)));

    let sums_low = vpaddlq_s16(products_low);
    let sums_high = vpaddlq_s16(products_high);

    vpaddq_s32(sums_low, sums_high)
}

pub unsafe fn dpbusd(i32s: int32x4_t, u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    vaddq_s32(i32s, dot_bytes(u8s, i8s))
}

pub unsafe fn double_dpbusd(
    i32s: int32x4_t, u8s1: int32x4_t, i8s1: int8x16_t, u8s2: int32x4_t, i8s2: int8x16_t,
) -> int32x4_t {
    let accum = vaddq_s32(dot_bytes(u8s1, i8s1), dot_bytes(u8s2, i8s2));
    vaddq_s32(i32s, accum)
}

pub unsafe fn horizontal_sum(x: [float32x4_t; 4]) -> f32 {
    let sum01 = vaddq_f32(x[0], x[1]);
    let sum23 = vaddq_f32(x[2], x[3]);
    let sum = vaddq_f32(sum01, sum23);

    let pair = vpadd_f32(vget_low_f32(sum), vget_high_f32(sum));
    let final_sum = vpadd_f32(pair, pair);

    vget_lane_f32::<0>(final_sum)
}

pub unsafe fn nnz_bitmask(x: int32x4_t) -> u16 {
    let cmp = vcgtq_s32(x, vdupq_n_s32(0));

    let mask0 = (vgetq_lane_u32::<0>(cmp) >> 31) & 1;
    let mask1 = ((vgetq_lane_u32::<1>(cmp) >> 31) & 1) << 1;
    let mask2 = ((vgetq_lane_u32::<2>(cmp) >> 31) & 1) << 2;
    let mask3 = ((vgetq_lane_u32::<3>(cmp) >> 31) & 1) << 3;

    (mask0 | mask1 | mask2 | mask3) as u16
}
