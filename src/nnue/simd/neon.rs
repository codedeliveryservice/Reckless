use std::{arch::aarch64::*, mem::size_of};

pub const F32_LANES: usize = size_of::<float32x4_t>() / size_of::<f32>();
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

#[allow(unused)]
unsafe fn dot_bytes(u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
    let u8s = vreinterpretq_u8_s32(u8s);

    let products_low = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_low_u8(u8s))), vmovl_s8(vget_low_s8(i8s)));
    let products_high = vmulq_s16(vreinterpretq_s16_u16(vmovl_u8(vget_high_u8(u8s))), vmovl_s8(vget_high_s8(i8s)));

    let sums_low = vpaddlq_s16(products_low);
    let sums_high = vpaddlq_s16(products_high);

    vpaddq_s32(sums_low, sums_high)
}

cfg_select! {
    target_feature = "dotprod" => {
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
    }
    _ => {
        pub unsafe fn dpbusd(i32s: int32x4_t, u8s: int32x4_t, i8s: int8x16_t) -> int32x4_t {
            vaddq_s32(i32s, dot_bytes(u8s, i8s))
        }
    }
}

pub unsafe fn double_dpbusd(
    i32s: int32x4_t, u8s1: int32x4_t, i8s1: int8x16_t, u8s2: int32x4_t, i8s2: int8x16_t,
) -> int32x4_t {
    dpbusd(dpbusd(i32s, u8s1, i8s1), u8s2, i8s2)
}

pub unsafe fn horizontal_sum(x: [float32x4_t; 4]) -> f32 {
    // The reduction order is important to prevent rounding differences
    // with the AVX2/512 implementations
    let sum02 = vaddq_f32(x[0], x[2]);
    let sum13 = vaddq_f32(x[1], x[3]);
    let sum = vaddq_f32(sum02, sum13);

    let pair = vadd_f32(vget_low_f32(sum), vget_high_f32(sum));

    vget_lane_f32::<0>(pair) + vget_lane_f32::<1>(pair)
}

pub unsafe fn nnz_bitmask(x: int32x4_t) -> u16 {
    let cmp = vcgtq_s32(x, vdupq_n_s32(0));

    let values: [u32; 4] = [1, 2, 4, 8];
    vaddvq_u32(vandq_u32(cmp, vld1q_u32(values.as_ptr()))) as u16
}
