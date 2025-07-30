use std::arch::x86_64::*;

use crate::{
    nnue::{
        accumulator::Accumulator, simd, Aligned, SparseEntry, DEQUANT_MULTIPLIER, FT_QUANT, FT_SHIFT, L1_SIZE, L2_SIZE,
        L3_SIZE, PARAMETERS,
    },
    types::Color,
};

pub unsafe fn activate_ft(
    accumulator: &Accumulator, nnz_table: &[SparseEntry], stm: Color,
) -> (Aligned<[u8; L1_SIZE]>, Aligned<[u16; L1_SIZE / 4]>, usize) {
    let mut output = Aligned::new([0; L1_SIZE]);

    let nnz_increment = _mm_set1_epi16(8);
    let mut nnz_base = _mm_setzero_si128();
    let mut nnz_indexes = Aligned::new([0; L1_SIZE / 4]);
    let mut nnz_count = 0;

    let zero = _mm256_setzero_si256();
    let one = _mm256_set1_epi16(FT_QUANT as i16);

    for flip in [0, 1] {
        let input = &accumulator.values[stm as usize ^ flip];

        for i in (0..L1_SIZE / 2).step_by(2 * simd::I16_LANES) {
            let lhs1 = *input.as_ptr().add(i).cast();
            let lhs2 = *input.as_ptr().add(i + simd::I16_LANES).cast();

            let rhs1 = *input.as_ptr().add(i + L1_SIZE / 2).cast();
            let rhs2 = *input.as_ptr().add(i + L1_SIZE / 2 + simd::I16_LANES).cast();

            let lhs1_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs1, zero), one);
            let lhs2_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs2, zero), one);

            let rhs1_clipped = _mm256_min_epi16(rhs1, one);
            let rhs2_clipped = _mm256_min_epi16(rhs2, one);

            let shifted1 = _mm256_slli_epi16::<{ 16 - FT_SHIFT }>(lhs1_clipped);
            let shifted2 = _mm256_slli_epi16::<{ 16 - FT_SHIFT }>(lhs2_clipped);

            let product1 = _mm256_mulhi_epi16(shifted1, rhs1_clipped);
            let product2 = _mm256_mulhi_epi16(shifted2, rhs2_clipped);

            let packed = _mm256_packus_epi16(product1, product2);
            let unpacked = _mm256_permute4x64_epi64::<0b11_01_10_00>(packed);

            *output.as_mut_ptr().add(i + flip * L1_SIZE / 2).cast() = unpacked;

            let mask = simd::nnz_bitmask(unpacked);
            let entry = nnz_table.get_unchecked(mask as usize);

            let store = nnz_indexes.as_mut_ptr().add(nnz_count).cast();
            _mm_storeu_si128(store, _mm_add_epi16(nnz_base, *entry.indexes.as_ptr().cast()));

            nnz_count += entry.count;
            nnz_base = _mm_add_epi16(nnz_base, nnz_increment);
        }
    }

    (output, nnz_indexes, nnz_count)
}

pub unsafe fn propagate_l1(ft_out: Aligned<[u8; L1_SIZE]>, nnz: &[u16]) -> Aligned<[f32; L2_SIZE]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = Aligned::new([_mm256_setzero_si256(); L2_SIZE / simd::F32_LANES]);

    let packed = std::slice::from_raw_parts(ft_out.as_ptr().cast::<i32>(), L1_SIZE / CHUNKS);

    let mut pairs = nnz.chunks_exact(2);

    for pair in &mut pairs {
        let index1 = *pair.get_unchecked(0) as usize;
        let index2 = *pair.get_unchecked(1) as usize;

        let input1 = _mm256_set1_epi32(*packed.get_unchecked(index1));
        let input2 = _mm256_set1_epi32(*packed.get_unchecked(index2));

        let weights1 = PARAMETERS.l1_weights.as_ptr().add(index1 * L2_SIZE * CHUNKS);
        let weights2 = PARAMETERS.l1_weights.as_ptr().add(index2 * L2_SIZE * CHUNKS);

        for j in (0..L2_SIZE).step_by(simd::F32_LANES) {
            let weights1 = weights1.add(j * CHUNKS).cast();
            let weights2 = weights2.add(j * CHUNKS).cast();

            let vector = &mut pre_activations[j / simd::F32_LANES];
            *vector = simd::double_dpbusd(*vector, input1, *weights1, input2, *weights2);
        }
    }

    if let Some(last) = pairs.remainder().first() {
        let index = *last as usize;
        let input = _mm256_set1_epi32(*packed.get_unchecked(index));
        let weights = PARAMETERS.l1_weights.as_ptr().add(index * L2_SIZE * CHUNKS);

        for j in (0..L2_SIZE).step_by(simd::F32_LANES) {
            let weights = weights.add(j * CHUNKS).cast();
            let vector = &mut pre_activations[j / simd::F32_LANES];
            *vector = simd::dpbusd(*vector, input, *weights);
        }
    }

    let mut output = Aligned::new([0.0; L2_SIZE]);

    let zero = _mm256_setzero_ps();
    let one = _mm256_set1_ps(1.0);
    let dequant = _mm256_set1_ps(DEQUANT_MULTIPLIER);

    for i in (0..L2_SIZE).step_by(simd::F32_LANES) {
        let biases = _mm256_load_ps(PARAMETERS.l1_biases.as_ptr().add(i).cast());
        let vector = _mm256_fmadd_ps(_mm256_cvtepi32_ps(pre_activations[i / simd::F32_LANES]), dequant, biases);
        *output.as_mut_ptr().add(i).cast() = _mm256_max_ps(_mm256_min_ps(vector, one), zero);
    }

    output
}

pub unsafe fn propagate_l2(l1_out: Aligned<[f32; L2_SIZE]>) -> Aligned<[f32; L3_SIZE]> {
    let mut output = PARAMETERS.l2_biases.clone();

    for i in 0..L2_SIZE {
        let input = _mm256_set1_ps(l1_out[i]);
        let weights = PARAMETERS.l2_weights[i].as_ptr();

        for j in (0..L3_SIZE).step_by(simd::F32_LANES) {
            let weights = weights.add(j).cast();
            let vector = output.as_mut_ptr().add(j).cast();
            *vector = _mm256_fmadd_ps(*weights, input, *vector);
        }
    }

    let zero = _mm256_setzero_ps();
    let one = _mm256_set1_ps(1.0);

    for i in (0..L3_SIZE).step_by(simd::F32_LANES) {
        let vector = output.as_mut_ptr().add(i).cast();
        *vector = _mm256_min_ps(_mm256_max_ps(*vector, zero), one);
    }

    output
}

pub unsafe fn propagate_l3(l2_out: Aligned<[f32; L3_SIZE]>) -> f32 {
    let input = l2_out.as_ptr();
    let weights = PARAMETERS.l3_weights.as_ptr();

    let mut output = _mm256_setzero_ps();

    for i in (0..L3_SIZE).step_by(simd::F32_LANES) {
        let a = weights.add(i).cast();
        let b = input.add(i).cast();
        output = _mm256_fmadd_ps(*a, *b, output);
    }

    simd::horizontal_sum(output) + PARAMETERS.l3_biases
}
