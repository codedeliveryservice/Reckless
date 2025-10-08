use std::arch::x86_64::*;

use crate::{
    nnue::{
        accumulator::Accumulator, simd, Aligned, SparseEntry, DEQUANT_MULTIPLIER, FT_QUANT, FT_SHIFT, L1_SIZE, L2_SIZE,
        L3_SIZE, PARAMETERS,
    },
    types::Color,
};

pub unsafe fn activate_ft(accumulator: &Accumulator, stm: Color) -> Aligned<[u8; L1_SIZE]> {
    let mut output = Aligned::new([0; L1_SIZE]);

    let zero = simd::zeroed();
    let one = simd::splat_i16(FT_QUANT as i16);

    for flip in [0, 1] {
        let input = &accumulator.values[stm as usize ^ flip];

        for i in (0..L1_SIZE / 2).step_by(2 * simd::I16_LANES) {
            let lhs1 = *input.as_ptr().add(i).cast();
            let lhs2 = *input.as_ptr().add(i + simd::I16_LANES).cast();

            let rhs1 = *input.as_ptr().add(i + L1_SIZE / 2).cast();
            let rhs2 = *input.as_ptr().add(i + L1_SIZE / 2 + simd::I16_LANES).cast();

            let lhs1_clipped = simd::clamp_i16(lhs1, zero, one);
            let lhs2_clipped = simd::clamp_i16(lhs2, zero, one);

            let rhs1_clipped = simd::min_i16(rhs1, one);
            let rhs2_clipped = simd::min_i16(rhs2, one);

            let shifted1 = simd::shift_left_i16::<{ 16 - FT_SHIFT }>(lhs1_clipped);
            let shifted2 = simd::shift_left_i16::<{ 16 - FT_SHIFT }>(lhs2_clipped);

            let product1 = simd::mul_high_i16(shifted1, rhs1_clipped);
            let product2 = simd::mul_high_i16(shifted2, rhs2_clipped);

            let packed = simd::packus(product1, product2);
            let unpacked = simd::permute(packed);

            *output.as_mut_ptr().add(i + flip * L1_SIZE / 2).cast() = unpacked;
        }
    }

    output
}

pub unsafe fn find_nnz(
    ft_out: &Aligned<[u8; L1_SIZE]>, nnz_table: &[SparseEntry],
) -> (Aligned<[u16; L1_SIZE / 4]>, usize) {
    let mut indexes = Aligned::new([0; L1_SIZE / 4]);
    let mut count = 0;

    let increment = _mm_set1_epi16(8);
    let mut base = _mm_setzero_si128();

    for i in (0..L1_SIZE).step_by(2 * simd::I16_LANES) {
        let mask = simd::nnz_bitmask(*ft_out.as_ptr().add(i).cast());

        for offset in (0..simd::I32_LANES).step_by(8) {
            let slice = (mask >> offset) & 0xFF;
            let entry = nnz_table.get_unchecked(slice as usize);

            let store = indexes.as_mut_ptr().add(count).cast();
            _mm_storeu_si128(store, _mm_add_epi16(base, *entry.indexes.as_ptr().cast()));

            count += entry.count;
            base = _mm_add_epi16(base, increment);
        }
    }

    (indexes, count)
}

pub unsafe fn propagate_l1(ft_out: Aligned<[u8; L1_SIZE]>, nnz: &[u16]) -> Aligned<[f32; L2_SIZE]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = Aligned::new([simd::zeroed(); L2_SIZE / simd::F32_LANES]);

    let packed = std::slice::from_raw_parts(ft_out.as_ptr().cast::<i32>(), L1_SIZE / CHUNKS);

    let mut pairs = nnz.chunks_exact(4);

    for pair in &mut pairs {
        let index1 = *pair.get_unchecked(0) as usize;
        let index2 = *pair.get_unchecked(1) as usize;
        let index3 = *pair.get_unchecked(2) as usize;
        let index4 = *pair.get_unchecked(3) as usize;

        let input1 = simd::splat_i32(*packed.get_unchecked(index1));
        let input2 = simd::splat_i32(*packed.get_unchecked(index2));
        let input3 = simd::splat_i32(*packed.get_unchecked(index3));
        let input4 = simd::splat_i32(*packed.get_unchecked(index4));

        let weights1 = PARAMETERS.l1_weights.as_ptr().add(index1 * L2_SIZE * CHUNKS);
        let weights2 = PARAMETERS.l1_weights.as_ptr().add(index2 * L2_SIZE * CHUNKS);
        let weights3 = PARAMETERS.l1_weights.as_ptr().add(index3 * L2_SIZE * CHUNKS);
        let weights4 = PARAMETERS.l1_weights.as_ptr().add(index4 * L2_SIZE * CHUNKS);

        for j in (0..L2_SIZE).step_by(simd::F32_LANES) {
            let w1 = weights1.add(j * CHUNKS).cast();
            let w2 = weights2.add(j * CHUNKS).cast();
            let w3 = weights3.add(j * CHUNKS).cast();
            let w4 = weights4.add(j * CHUNKS).cast();

            let vector = &mut pre_activations[j / simd::F32_LANES];

            *vector = simd::double_dpbusd(*vector, input1, *w1, input2, *w2);
            *vector = simd::double_dpbusd(*vector, input3, *w3, input4, *w4);
        }
    }

    for &index in pairs.remainder() {
        let index = index as usize;
        let input = simd::splat_i32(*packed.get_unchecked(index));
        let weights = PARAMETERS.l1_weights.as_ptr().add(index * L2_SIZE * CHUNKS);

        for j in (0..L2_SIZE).step_by(simd::F32_LANES) {
            let weights = weights.add(j * CHUNKS).cast();
            let vector = &mut pre_activations[j / simd::F32_LANES];
            *vector = simd::dpbusd(*vector, input, *weights);
        }
    }

    let mut output = Aligned::new([0.0; L2_SIZE]);

    let zero = simd::zero_f32();
    let one = simd::splat_f32(1.0);
    let dequant = simd::splat_f32(DEQUANT_MULTIPLIER);

    for i in (0..L2_SIZE).step_by(simd::F32_LANES) {
        let biases = *PARAMETERS.l1_biases.as_ptr().add(i).cast();
        let vector = simd::mul_add_f32(simd::convert_to_f32(pre_activations[i / simd::F32_LANES]), dequant, biases);
        *output.as_mut_ptr().add(i).cast() = simd::clamp_f32(vector, zero, one);
    }

    output
}

pub unsafe fn propagate_l2(l1_out: Aligned<[f32; L2_SIZE]>) -> Aligned<[f32; L3_SIZE]> {
    let mut output = PARAMETERS.l2_biases.clone();

    for i in 0..L2_SIZE {
        let input = simd::splat_f32(l1_out[i]);
        let weights = PARAMETERS.l2_weights[i].as_ptr();

        for j in (0..L3_SIZE).step_by(simd::F32_LANES) {
            let weights = weights.add(j).cast();
            let vector = output.as_mut_ptr().add(j).cast();
            *vector = simd::mul_add_f32(*weights, input, *vector);
        }
    }

    let zero = simd::zero_f32();
    let one = simd::splat_f32(1.0);

    for i in (0..L3_SIZE).step_by(simd::F32_LANES) {
        let vector = output.as_mut_ptr().add(i).cast();
        *vector = simd::clamp_f32(*vector, zero, one);
    }

    output
}

pub unsafe fn propagate_l3(l2_out: Aligned<[f32; L3_SIZE]>) -> f32 {
    const LANES: usize = 16 / simd::F32_LANES;

    let input = l2_out.as_ptr();
    let weights = PARAMETERS.l3_weights.as_ptr();

    let mut output = [simd::zero_f32(); LANES];

    for (lane, result) in output.iter_mut().enumerate() {
        for i in (0..L3_SIZE).step_by(LANES * simd::F32_LANES) {
            let a = weights.add(i + lane * simd::F32_LANES).cast();
            let b = input.add(i + lane * simd::F32_LANES).cast();

            *result = simd::mul_add_f32(*a, *b, *result);
        }
    }

    simd::horizontal_sum(output) + PARAMETERS.l3_biases
}
