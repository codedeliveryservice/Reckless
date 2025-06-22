use std::arch::x86_64::*;

use crate::{
    board::Board,
    types::{Color, Move, PieceType, MAX_PLY},
};

use accumulator::{Accumulator, AccumulatorCache};

#[cfg(all(target_feature = "avx2", not(target_arch = "aarch64")))]
use avx2 as simd;
#[cfg(target_arch = "aarch64")]
use fallback as simd;
#[cfg(all(not(target_feature = "avx2"), not(target_arch = "aarch64")))]
use ssse3 as simd;

mod accumulator;

#[cfg(all(target_feature = "avx2", not(target_arch = "aarch64")))]
mod avx2;
#[cfg(target_arch = "aarch64")]
mod fallback;
#[cfg(all(not(target_feature = "avx2"), not(target_arch = "aarch64")))]
mod ssse3;

const INPUT_BUCKETS: usize = 4;

const FT_SIZE: usize = 768;
const L1_SIZE: usize = 1024;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;

const FT_QUANT: i32 = 255;
const L1_QUANT: i32 = 64;
const FT_SHIFT: i32 = 8;

const NETWORK_SCALE: i32 = 400;

#[rustfmt::skip]
const BUCKETS: [usize; 64] = [
    0, 0, 1, 1, 1, 1, 0, 0,
    2, 2, 2, 2, 2, 2, 2, 2,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3,
];

#[derive(Clone)]
pub struct Network {
    index: usize,
    stack: Box<[Accumulator]>,
    cache: AccumulatorCache,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv.is_some());

        self.index += 1;
        self.stack[self.index].accurate = [false; 2];
        self.stack[self.index].delta.mv = mv;
        self.stack[self.index].delta.piece = board.piece_on(mv.from());
        self.stack[self.index].delta.captured = board.piece_on(mv.to());
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn full_refresh(&mut self, board: &Board) {
        self.stack[self.index].refresh(board, Color::White, &mut self.cache);
        self.stack[self.index].refresh(board, Color::Black, &mut self.cache);
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        debug_assert!(self.stack[0].accurate == [true; 2]);

        for pov in [Color::White, Color::Black] {
            if self.stack[self.index].accurate[pov] {
                continue;
            }

            if self.can_update(pov) {
                self.update_accumulator(board, pov);
            } else {
                self.refresh(board, pov);
            }
        }

        self.output_transformer(board)
    }

    fn refresh(&mut self, board: &Board, pov: Color) {
        self.stack[self.index].refresh(board, pov, &mut self.cache);
    }

    fn update_accumulator(&mut self, board: &Board, pov: Color) {
        let king = board.king_square(pov);
        let index = (0..self.index).rfind(|&i| self.stack[i].accurate[pov]).unwrap();

        for i in index..self.index {
            if let (prev, [current, ..]) = self.stack.split_at_mut(i + 1) {
                current.update(&prev[i], king, pov);
            }
        }
    }

    fn can_update(&self, pov: Color) -> bool {
        for i in (0..=self.index).rev() {
            let delta = self.stack[i].delta;

            let (from, to) = match delta.piece.piece_color() {
                Color::White => (delta.mv.from(), delta.mv.to()),
                Color::Black => (delta.mv.from() ^ 56, delta.mv.to() ^ 56),
            };

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && ((from.file() >= 4) != (to.file() >= 4) || BUCKETS[from] != BUCKETS[to])
            {
                return false;
            }

            if self.stack[i].accurate[pov] {
                return true;
            }
        }

        false
    }

    unsafe fn activate_ft(&self, board: &Board) -> Aligned<[u8; L1_SIZE]> {
        const VECTOR_WIDTH: usize = 16;

        let mut output = Aligned { data: [0u8; L1_SIZE] };

        let zero = _mm256_setzero_si256();
        let one = _mm256_set1_epi16(FT_QUANT as i16);

        for flip in [0, 1] {
            let acc = &self.stack[self.index].values[board.side_to_move() as usize ^ flip];

            for i in (0..L1_SIZE / 2).step_by(2 * VECTOR_WIDTH) {
                let lhs1 = _mm256_load_si256(acc.as_ptr().add(i).cast());
                let lhs2 = _mm256_load_si256(acc.as_ptr().add(i + VECTOR_WIDTH).cast());

                let rhs1 = _mm256_load_si256(acc.as_ptr().add(i + L1_SIZE / 2).cast());
                let rhs2 = _mm256_load_si256(acc.as_ptr().add(i + L1_SIZE / 2 + VECTOR_WIDTH).cast());

                let lhs1_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs1, zero), one);
                let lhs2_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs2, zero), one);

                let rhs1_clipped = _mm256_min_epi16(_mm256_max_epi16(rhs1, zero), one);
                let rhs2_clipped = _mm256_min_epi16(_mm256_max_epi16(rhs2, zero), one);

                let product1 = _mm256_mullo_epi16(lhs1_clipped, rhs1_clipped);
                let product2 = _mm256_mullo_epi16(lhs2_clipped, rhs2_clipped);

                let shifted1 = _mm256_srli_epi16::<FT_SHIFT>(product1);
                let shifted2 = _mm256_srli_epi16::<FT_SHIFT>(product2);

                let packed = _mm256_packus_epi16(shifted1, shifted2);
                let unpacked = _mm256_permute4x64_epi64::<0b11_01_10_00>(packed);

                *output.data.as_mut_ptr().add(i + flip * L1_SIZE / 2).cast() = unpacked;
            }
        }

        output
    }

    fn output_transformer(&self, board: &Board) -> i32 {
        let ft_out = unsafe { self.activate_ft(board) };
        let l1_out = unsafe { propagate_l1(ft_out) };

        let mut hl3 = [0.0; L3_SIZE];

        for i in 0..L2_SIZE {
            for j in 0..L3_SIZE {
                hl3[j] += PARAMETERS.l2_weights[i][j] * l1_out[i];
            }
        }

        for j in 0..L3_SIZE {
            hl3[j] += PARAMETERS.l2_biases[j];
            hl3[j] = hl3[j].clamp(0.0, 1.0);
        }

        let mut output = PARAMETERS.l3_biases;
        for i in 0..L3_SIZE {
            output += PARAMETERS.l3_weights[i] * hl3[i];
        }

        (output * NETWORK_SCALE as f32) as i32
    }
}

unsafe fn propagate_l1(ft_out: Aligned<[u8; L1_SIZE]>) -> Aligned<[f32; L2_SIZE]> {
    const VECTOR_WIDTH: usize = 8;
    const DEQUANT_MULTIPLIER: f32 = (1 << FT_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

    let mut pre_activations = Aligned { data: [_mm256_setzero_si256(); L2_SIZE / VECTOR_WIDTH] };

    for in_index in 0..L1_SIZE {
        let ft_value = _mm256_set1_epi32(ft_out[in_index] as i32);

        for out_index in 0..L2_SIZE / VECTOR_WIDTH {
            let weights = PARAMETERS.l1_weights[in_index].as_ptr().add(out_index * VECTOR_WIDTH).cast();
            let weights = _mm256_cvtepi8_epi32(*weights);
            let product = _mm256_mullo_epi32(ft_value, weights);

            pre_activations[out_index] = _mm256_add_epi32(pre_activations[out_index], product);
        }
    }

    let mut output = Aligned { data: [0.0; L2_SIZE] };

    let zero = _mm256_setzero_ps();
    let one = _mm256_set1_ps(1.0);

    let dequant = _mm256_set1_ps(DEQUANT_MULTIPLIER);

    for i in 0..L2_SIZE / 8 {
        let biases = _mm256_load_ps(PARAMETERS.l1_biases.as_ptr().add(i * 8).cast());
        let vector = _mm256_fmadd_ps(_mm256_cvtepi32_ps(pre_activations[i]), dequant, biases);

        *output.as_mut_ptr().add(i * 8).cast() = _mm256_max_ps(_mm256_min_ps(vector, one), zero);
    }

    output
}

impl Default for Network {
    fn default() -> Self {
        Self {
            index: 0,
            stack: vec![Accumulator::new(); MAX_PLY].into_boxed_slice(),
            cache: AccumulatorCache::default(),
        }
    }
}

#[repr(C)]
struct Parameters {
    ft_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * FT_SIZE]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[[i8; L2_SIZE]; L1_SIZE]>,
    l1_biases: Aligned<[f32; L2_SIZE]>,
    l2_weights: Aligned<[[f32; L3_SIZE]; L2_SIZE]>,
    l2_biases: Aligned<[f32; L3_SIZE]>,
    l3_weights: Aligned<[f32; L3_SIZE]>,
    l3_biases: f32,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

#[repr(align(64))]
#[derive(Copy, Clone)]
struct Aligned<T> {
    data: T,
}

impl<T> std::ops::Deref for Aligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
