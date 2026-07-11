use crate::{
    nnue::{
        Aligned, FT_QUANT, FT_SHIFT, L1_SIZE, L2_SIZE, L3_SIZE, Parameters, SparseEntry, TAIL_ACT_QUANT, TAIL_SHIFT,
        accumulator::{PstAccumulator, ThreatAccumulator},
    },
    types::Color,
};

pub fn activate_ft(pst: &PstAccumulator, threat: &ThreatAccumulator, stm: Color) -> Aligned<[u8; L1_SIZE]> {
    let mut output = Aligned::new([0; L1_SIZE]);

    for flip in [0, 1] {
        let pst_input = &pst.values[stm as usize ^ flip];
        let threat_input = &threat.values[stm as usize ^ flip];

        for i in 0..L1_SIZE / 2 {
            let left = (pst_input[i] + threat_input[i]).clamp(0, FT_QUANT as i16);
            let right = (pst_input[i + L1_SIZE / 2] + threat_input[i + L1_SIZE / 2]).clamp(0, FT_QUANT as i16);

            output[i + flip * L1_SIZE / 2] = ((left as i32 * right as i32) >> FT_SHIFT) as u8;
        }
    }

    output
}

pub unsafe fn propagate_l1(
    ft_out: &Aligned<[u8; L1_SIZE]>, nnz: &[u16], bucket: usize, parameters: &Parameters,
) -> Aligned<[i32; L2_SIZE]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = Aligned::new([0i32; L2_SIZE]);

    let packed = std::slice::from_raw_parts(ft_out.as_ptr() as *const i32, L1_SIZE / CHUNKS);

    for i in 0..nnz.len() {
        let index = *nnz.get_unchecked(i) as usize;
        let input = packed.get_unchecked(index);
        let weights = &parameters.l1_weights[bucket][index * L2_SIZE * CHUNKS..];

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

    pre_activations
}

pub fn propagate_l2(
    l1_out: &Aligned<[i16; L2_SIZE]>, bucket: usize, parameters: &Parameters,
) -> Aligned<[i16; L3_SIZE]> {
    let mut accumulators = parameters.l2_biases[bucket];

    for p in 0..L2_SIZE / 2 {
        let a = l1_out[2 * p] as i32;
        let b = l1_out[2 * p + 1] as i32;
        let row = &parameters.l2_weights[bucket][p];

        for j in 0..L3_SIZE {
            accumulators[j] += a * row[2 * j] as i32 + b * row[2 * j + 1] as i32;
        }
    }

    let mut output = Aligned::new([0; L3_SIZE]);

    for j in 0..L3_SIZE {
        let activation = (accumulators[j] + (1 << (TAIL_SHIFT - 1))) >> TAIL_SHIFT;
        output[j] = activation.clamp(0, TAIL_ACT_QUANT) as i16;
    }

    output
}

pub fn propagate_l3(l2_out: &Aligned<[i16; L3_SIZE]>, bucket: usize, parameters: &Parameters) -> i32 {
    let mut accumulator = 0;

    for i in 0..L3_SIZE {
        accumulator += l2_out[i] as i32 * parameters.l3_weights[bucket][i] as i32;
    }

    accumulator + parameters.l3_biases[bucket]
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
