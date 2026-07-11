use crate::nnue::{
    Aligned, FT_QUANT, FT_SHIFT, L1_QUANT, L2_SIZE, NETWORK_SCALE, Parameters, TAIL_ACT_QUANT, TAIL_SHIFT,
};

const DEQUANT_DIVISOR: i64 = (FT_QUANT * FT_QUANT * L1_QUANT) as i64;
const OUTPUT_DIVISOR: i64 = (TAIL_ACT_QUANT as i64) << TAIL_SHIFT;

const L1_SHIFT: i64 = 30;
const L1_MULTIPLIER: i64 =
    ((1 << (L1_SHIFT + FT_SHIFT as i64)) * TAIL_ACT_QUANT as i64 + DEQUANT_DIVISOR / 2) / DEQUANT_DIVISOR;

#[inline]
pub fn activate_l1(
    pre_activations: &Aligned<[i32; L2_SIZE]>, bucket: usize, parameters: &Parameters,
) -> Aligned<[i16; L2_SIZE]> {
    let mut output = Aligned::new([0; L2_SIZE]);

    for i in 0..L2_SIZE {
        let raw = pre_activations[i] as i64 * L1_MULTIPLIER + parameters.l1_biases[bucket][i];
        let activation = (raw + (1 << (L1_SHIFT - 1))) >> L1_SHIFT;

        output[i] = activation.clamp(0, TAIL_ACT_QUANT as i64) as i16;
    }

    output
}

#[inline]
pub fn scale_output(l3_out: i32) -> i32 {
    // Truncates toward zero, like the `as i32` cast the f32 tail used.
    (l3_out as i64 * NETWORK_SCALE as i64 / OUTPUT_DIVISOR) as i32
}
