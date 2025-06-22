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

    fn output_transformer(&self, board: &Board) -> i32 {
        const FT_SHIFT: usize = 8;
        const QUANT_FACTOR: f32 = (1 << FT_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

        let accumulators = &self.stack[self.index].values.data;

        let mut hl1 = [0; L1_SIZE];

        for flip in [0, 1] {
            let accumulator = &accumulators[board.side_to_move() as usize ^ flip];

            for i in 0..L1_SIZE / 2 {
                let left = accumulator[i].clamp(0, FT_QUANT as i16);
                let right = accumulator[i + L1_SIZE / 2].clamp(0, FT_QUANT as i16);

                hl1[i + flip * L1_SIZE / 2] = ((left as i32 * right as i32) >> FT_SHIFT) as u8;
            }
        }

        let mut sums = [0; L2_SIZE];

        for i in 0..L1_SIZE {
            for j in 0..L2_SIZE {
                sums[j] += PARAMETERS.l1_weights[i][j] as i32 * hl1[i] as i32;
            }
        }

        let mut hl2 = [0.0; L2_SIZE];

        for i in 0..L2_SIZE {
            hl2[i] = (sums[i] as f32 * QUANT_FACTOR + PARAMETERS.l1_biases[i] as f32).clamp(0.0, 1.0);
        }

        let mut hl3 = [0.0; L3_SIZE];

        for i in 0..L2_SIZE {
            for j in 0..L3_SIZE {
                hl3[j] += PARAMETERS.l2_weights[i][j] * hl2[i];
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
