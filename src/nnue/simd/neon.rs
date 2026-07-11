use std::{arch::aarch64::*, mem::size_of};

pub const I32_LANES: usize = size_of::<int32x4_t>() / size_of::<i32>();
pub const I16_LANES: usize = size_of::<int16x8_t>() / size_of::<i16>();
pub const MUL_HI_SHIFT: i32 = 1;

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
    // doubles the result, so one of the inputs must be preshifted
    vqdmulhq_s16(a, b)
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

pub unsafe fn madd_i16(a: int32x4_t, b: int16x8_t) -> int32x4_t {
    let a = vreinterpretq_s16_s32(a);

    let low = vmull_s16(vget_low_s16(a), vget_low_s16(b));
    let high = vmull_high_s16(a, b);
    vpaddq_s32(low, high)
}

pub unsafe fn add_i32(a: int32x4_t, b: int32x4_t) -> int32x4_t {
    vaddq_s32(a, b)
}

pub unsafe fn shift_right_i32<const SHIFT: i32>(a: int32x4_t) -> int32x4_t {
    vshrq_n_s32::<SHIFT>(a)
}

pub unsafe fn clamp_i32(x: int32x4_t, min: int32x4_t, max: int32x4_t) -> int32x4_t {
    vmaxq_s32(vminq_s32(x, max), min)
}

pub unsafe fn pack_i32(a: int32x4_t, b: int32x4_t) -> int16x8_t {
    vcombine_s16(vqmovn_s32(a), vqmovn_s32(b))
}

pub unsafe fn horizontal_sum_i32(x: int32x4_t) -> i32 {
    vaddvq_s32(x)
}

#[allow(unused)]
unsafe fn dot_bytes(u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    let u8s = vreinterpretq_u8_s32(u8s);

    let products_low = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_low_u8(u8s))), vmovl_s8(vget_low_s8(i8s)));
    let products_high = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_high_u8(u8s))), vmovl_s8(vget_high_s8(i8s)));

    let sums_low = vpaddlq_s16(products_low);
    let sums_high = vpaddlq_s16(products_high);

    vpaddq_s32(sums_low, sums_high)
}

#[cfg(target_feature = "dotprod")]
pub unsafe fn dpbusd(mut i32s: int32x4_t, u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    // Nightly only equivalent:
    // vdotq_s32(i32s, vreinterpretq_s8_s32(u8s), i8s)
    std::arch::asm!(
        "sdot {acc:v}.4s, {src1:v}.16b, {src2:v}.16b",
        acc  = inout(vreg) i32s,
        src1 = in(vreg) u8s,
        src2 = in(vreg) i8s,
        options(pure, nomem, nostack)
    );
    i32s
}

#[cfg(not(target_feature = "dotprod"))]
pub unsafe fn dpbusd(i32s: int32x4_t, u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    vaddq_s32(i32s, dot_bytes(u8s, i8s))
}

pub unsafe fn double_dpbusd(
    i32s: int32x4_t, u8s1: int32x4_t, i8s1: int8x16_t, u8s2: int32x4_t, i8s2: int8x16_t,
) -> int32x4_t {
    dpbusd(dpbusd(i32s, u8s1, i8s1), u8s2, i8s2)
}

pub unsafe fn nnz_bitmask(x: int32x4_t) -> u16 {
    let cmp = vcgtq_s32(x, vdupq_n_s32(0));

    let values: [u32; 4] = [1, 2, 4, 8];
    vaddvq_u32(vandq_u32(cmp, vld1q_u32(values.as_ptr()))) as u16
}
