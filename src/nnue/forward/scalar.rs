use crate::{
    nnue::{
        accumulator::Accumulator, Aligned, SparseEntry, DEQUANT_MULTIPLIER, FT_QUANT, FT_SHIFT, L1_SIZE, L2_SIZE,
        L3_SIZE, PARAMETERS,
    },
    types::Color,
};

pub fn activate_ft(accumulator: &Accumulator, stm: Color) -> Aligned<[u8; L1_SIZE]> {
    let mut output = Aligned::new([0; L1_SIZE]);

    for flip in [0, 1] {
        let input = &accumulator.values[stm as usize ^ flip];

        for i in 0..L1_SIZE / 2 {
            let left = input[i].clamp(0, FT_QUANT as i16);
            let right = input[i + L1_SIZE / 2].clamp(0, FT_QUANT as i16);

            output[i + flip * L1_SIZE / 2] = ((left as i32 * right as i32) >> FT_SHIFT) as u8;
        }
    }

    output
}

pub unsafe fn find_nnz(ft_out: &Aligned<[u8; L1_SIZE]>, _: &[SparseEntry]) -> (Aligned<[u16; L1_SIZE / 4]>, usize) {
    let mut indexes = Aligned::new([0; L1_SIZE / 4]);
    let mut count = 0;

    for i in 0..L1_SIZE / 4 {
        let mut nonzero = 0;

        for j in 0..4 {
            nonzero |= ft_out[i * 4 + j];
        }

        if nonzero != 0 {
            indexes[count] = i as u16;
            count += 1;
        }
    }

    (indexes, count)
}

pub unsafe fn propagate_l1(ft_out: Aligned<[u8; L1_SIZE]>, nnz: &[u16]) -> Aligned<[f32; L2_SIZE]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = [0i32; L2_SIZE];

    let packed = std::slice::from_raw_parts(ft_out.as_ptr() as *const i32, L1_SIZE / CHUNKS);

    for i in 0..nnz.len() {
        let index = *nnz.get_unchecked(i) as usize;
        let input = packed.get_unchecked(index);
        let weights = &PARAMETERS.l1_weights[index * L2_SIZE * CHUNKS..];

        for j in 0..L2_SIZE {
            let mut vector = 0;

            for k in 0..CHUNKS {
                let byte = (input >> (k * 8)) & 0xFF;
                let weight = weights[j * CHUNKS + k];

                vector += (byte as i16) * (weight as i16);
            }

            pre_activations[j] += vector as i32;
        }
    }

    let mut output = Aligned::new([0.0; L2_SIZE]);

    for i in 0..L2_SIZE {
        output[i] = (pre_activations[i] as f32 * DEQUANT_MULTIPLIER + PARAMETERS.l1_biases[i]).clamp(0.0, 1.0);
    }

    output
}

pub fn propagate_l2(l1_out: Aligned<[f32; L2_SIZE]>) -> Aligned<[f32; L3_SIZE]> {
    let mut output = Aligned::new([0.0; L3_SIZE]);

    for i in 0..L2_SIZE {
        for j in 0..L3_SIZE {
            output[j] += PARAMETERS.l2_weights[i][j] * l1_out[i];
        }
    }

    for i in 0..L3_SIZE {
        output[i] += PARAMETERS.l2_biases[i];
        output[i] = output[i].clamp(0.0, 1.0);
    }
    output
}

pub fn propagate_l3(l2_out: Aligned<[f32; L3_SIZE]>) -> f32 {
    let mut output = 0.0;
    for i in 0..L3_SIZE {
        output = PARAMETERS.l3_weights[i].mul_add(l2_out[i], output);
    }
    output + PARAMETERS.l3_biases
}
