pub fn forward(accumulator: &[i16], weights: &[i16]) -> i32 {
    #[cfg(target_feature = "avx2")]
    {
        unsafe { avx2::forward(accumulator, weights) }
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        scalar::forward(accumulator, weights)
    }
}

#[cfg(target_feature = "avx2")]
mod avx2 {
    use std::arch::x86_64::*;

    use crate::nnue::{HIDDEN_SIZE, L0_SCALE};

    const CHUNK_SIZE: usize = 16;

    pub unsafe fn forward(accumulator: &[i16], weights: &[i16]) -> i32 {
        let mut vector = _mm256_setzero_si256();
        let min = _mm256_setzero_si256();
        let max = _mm256_set1_epi16(L0_SCALE as i16);

        for i in (0..HIDDEN_SIZE).step_by(CHUNK_SIZE) {
            let acc = _mm256_load_si256(accumulator.as_ptr().add(i).cast());
            let acc = _mm256_min_epi16(_mm256_max_epi16(acc, min), max);

            let w = _mm256_load_si256(weights.as_ptr().add(i).cast());
            let product = _mm256_madd_epi16(_mm256_mullo_epi16(acc, w), acc);

            vector = _mm256_add_epi32(vector, product);
        }

        let upper_half = _mm256_extracti128_si256::<1>(vector);
        let lower_half = _mm256_castsi256_si128(vector);

        let sum_128 = _mm_add_epi32(upper_half, lower_half);
        let sum_64 = _mm_add_epi32(_mm_unpackhi_epi64(sum_128, sum_128), sum_128);

        let shuffled = _mm_shuffle_epi32::<1>(sum_64);
        let sum = _mm_add_epi32(shuffled, sum_64);

        _mm_cvtsi128_si32(sum)
    }
}

#[cfg(not(target_feature = "avx2"))]
mod scalar {
    use crate::nnue::{HIDDEN_SIZE, L0_SCALE};

    pub fn forward(accumulator: &[i16], weights: &[i16]) -> i32 {
        let mut output = 0;
        for i in 0..HIDDEN_SIZE {
            output += screlu(i32::from(accumulator[i])) * i32::from(weights[i]);
        }
        output
    }

    fn screlu(x: i32) -> i32 {
        let v = x.clamp(0, L0_SCALE);
        v * v
    }
}
