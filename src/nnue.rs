use crate::{
    board::Board,
    types::{Color, Move, PieceType, MAX_PLY},
};

use accumulator::Accumulator;

#[cfg(target_feature = "avx2")]
use avx2 as simd;
#[cfg(not(target_feature = "avx2"))]
use ssse3 as simd;

mod accumulator;

#[cfg(target_feature = "avx2")]
mod avx2;
#[cfg(not(target_feature = "avx2"))]
mod ssse3;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 768;

const NETWORK_SCALE: i32 = 400;
const NETWORK_QA: i32 = 255;
const NETWORK_QB: i32 = 64;

#[derive(Clone)]
pub struct Network {
    index: usize,
    stack: Box<[Accumulator]>,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv.is_valid());

        self.index += 1;
        self.stack[self.index].accurate = false;
        self.stack[self.index].delta.mv = mv;
        self.stack[self.index].delta.piece = board.piece_on(mv.from());
        self.stack[self.index].delta.captured = board.piece_on(mv.to());
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn refresh(&mut self, board: &Board) {
        self.stack[self.index].refresh(board);
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        debug_assert!(self.stack[0].accurate);

        if !self.stack[self.index].accurate {
            if self.can_update() {
                self.update_accumulators(board);
            } else {
                self.refresh(board);
            }
        }

        let output = self.output_transformer(board) / NETWORK_QA + PARAMETERS.output_bias as i32;
        output * NETWORK_SCALE / (NETWORK_QA * NETWORK_QB)
    }

    fn update_accumulators(&mut self, board: &Board) {
        let wking = board.king_square(Color::White);
        let bking = board.king_square(Color::Black);
        let index = (0..self.index).rfind(|&i| self.stack[i].accurate).unwrap();

        for i in index..self.index {
            if let (prev, [current, ..]) = self.stack.split_at_mut(i + 1) {
                current.update(&prev[i], wking, bking);
            }
        }
    }

    fn can_update(&self) -> bool {
        for i in (0..=self.index).rev() {
            let delta = self.stack[i].delta;

            if delta.piece.piece_type() == PieceType::King
                && ((delta.mv.from()).file() >= 4) != ((delta.mv.to()).file() >= 4)
            {
                return false;
            }

            if self.stack[i].accurate {
                return true;
            }
        }

        false
    }

    fn output_transformer(&mut self, board: &Board) -> i32 {
        let accumulators = &self.stack[self.index];

        let min = simd::zero();
        let max = simd::splat(NETWORK_QA as i16);

        let mut vector = simd::zero();

        for flip in [0, 1] {
            let accumulator = &accumulators.values[board.side_to_move() as usize ^ flip];
            let weights = &PARAMETERS.output_weights[flip];

            for i in (0..HIDDEN_SIZE).step_by(simd::VECTOR_WIDTH) {
                let input = unsafe { *(accumulator[i..].as_ptr().cast()) };
                let weights = unsafe { *(weights[i..].as_ptr().cast()) };

                let v = simd::min(simd::max(input, min), max);
                let w = simd::mullo(v, weights);
                vector = simd::add_i32(vector, simd::dot(w, v));
            }
        }

        simd::horizontal_sum(vector)
    }
}

impl Default for Network {
    fn default() -> Self {
        Self {
            index: 0,
            stack: vec![Accumulator::new(); MAX_PLY].into_boxed_slice(),
        }
    }
}

#[repr(C)]
struct Parameters {
    ft_weights: Aligned<[[i16; HIDDEN_SIZE]; INPUT_SIZE]>,
    ft_biases: Aligned<[i16; HIDDEN_SIZE]>,
    output_weights: Aligned<[[i16; HIDDEN_SIZE]; 2]>,
    output_bias: i16,
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
