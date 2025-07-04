use std::arch::x86_64::*;

use crate::{
    board::Board,
    types::{Color, Move, PieceType, MAX_PLY},
};

use accumulator::{Accumulator, AccumulatorCache};
use avx2 as simd;

mod accumulator;
mod avx2;

const NETWORK_SCALE: i32 = 400;

const INPUT_BUCKETS: usize = 4;

const FT_SIZE: usize = 768;
const L1_SIZE: usize = 1024;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;

const LHS_FT_QUANT: i32 = 255;
const RHS_FT_QUANT: i32 = 510;

const L1_QUANT: i32 = 128;

const FT_SHIFT: i32 = 9;

const DEQUANT_MULTIPLIER: f32 = (1 << FT_SHIFT) as f32 / (LHS_FT_QUANT * RHS_FT_QUANT * L1_QUANT) as f32;

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

#[repr(align(16))]
#[derive(Clone, Copy)]
struct SparseEntry {
    indexes: [u16; 8],
    count: usize,
}

#[derive(Clone)]
pub struct Network {
    index: usize,
    stack: Box<[Accumulator]>,
    cache: AccumulatorCache,
    nnz_table: Box<[SparseEntry]>,
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

    fn output_transformer(&self, board: &Board) -> i32 {
        unsafe {
            let (ft_out, nnz_indexes, nnz_count) =
                activate_ft(&self.stack[self.index], &self.nnz_table, board.side_to_move());

            let l1_out = propagate_l1(ft_out, &nnz_indexes[..nnz_count]);
            let l2_out = propagate_l2(l1_out);
            let l3_out = propagate_l3(l2_out);

            (l3_out * NETWORK_SCALE as f32) as i32
        }
    }
}

unsafe fn activate_ft(
    accumulator: &Accumulator, nnz_table: &[SparseEntry], stm: Color,
) -> (Aligned<[u8; L1_SIZE]>, Aligned<[u16; L1_SIZE / 4]>, usize) {
    let mut output = Aligned::new([0; L1_SIZE]);

    let nnz_increment = _mm_set1_epi16(8);
    let mut nnz_base = _mm_setzero_si128();
    let mut nnz_indexes = Aligned::new([0; L1_SIZE / 4]);
    let mut nnz_count = 0;

    let zero = _mm256_setzero_si256();
    let one = _mm256_set1_epi16(LHS_FT_QUANT as i16);
    let two = _mm256_set1_epi16(RHS_FT_QUANT as i16);

    for flip in [0, 1] {
        let input = &accumulator.values[stm as usize ^ flip];

        for i in (0..L1_SIZE / 2).step_by(2 * simd::I16_LANES) {
            let lhs1 = *input.as_ptr().add(i).cast();
            let lhs2 = *input.as_ptr().add(i + simd::I16_LANES).cast();

            let rhs1 = *input.as_ptr().add(i + L1_SIZE / 2).cast();
            let rhs2 = *input.as_ptr().add(i + L1_SIZE / 2 + simd::I16_LANES).cast();

            let lhs1_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs1, zero), one);
            let lhs2_clipped = _mm256_min_epi16(_mm256_max_epi16(lhs2, zero), one);

            let rhs1_clipped = _mm256_min_epi16(rhs1, two);
            let rhs2_clipped = _mm256_min_epi16(rhs2, two);

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

unsafe fn propagate_l1(ft_out: Aligned<[u8; L1_SIZE]>, nnz: &[u16]) -> Aligned<[f32; L2_SIZE]> {
    const CHUNKS: usize = 4;

    let mut pre_activations = Aligned::new([_mm256_setzero_si256(); L2_SIZE / simd::F32_LANES]);

    let packed = std::slice::from_raw_parts(ft_out.as_ptr().cast::<i32>(), L1_SIZE / CHUNKS);

    for i in 0..nnz.len() {
        let index = *nnz.get_unchecked(i) as usize;
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

unsafe fn propagate_l2(l1_out: Aligned<[f32; L2_SIZE]>) -> Aligned<[f32; L3_SIZE]> {
    let mut output = PARAMETERS.l2_biases;

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

unsafe fn propagate_l3(l2_out: Aligned<[f32; L3_SIZE]>) -> f32 {
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

impl Default for Network {
    fn default() -> Self {
        let mut nnz_table = vec![SparseEntry { indexes: [0; 8], count: 0 }; 256];

        for (byte, entry) in nnz_table.iter_mut().enumerate() {
            let mut count = 0;

            for bit in 0..8 {
                if (byte & (1 << bit)) != 0 {
                    entry.indexes[count] = bit as u16;
                    count += 1;
                }
            }

            entry.count = count;
        }

        Self {
            index: 0,
            stack: vec![Accumulator::new(); MAX_PLY].into_boxed_slice(),
            cache: AccumulatorCache::default(),
            nnz_table: nnz_table.into_boxed_slice(),
        }
    }
}

#[repr(C)]
struct Parameters {
    ft_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * FT_SIZE]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[i8; L2_SIZE * L1_SIZE]>,
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

impl<T> Aligned<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
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
