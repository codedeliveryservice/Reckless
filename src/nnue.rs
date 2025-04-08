use crate::{
    board::Board,
    types::{Color, Move, PieceType, MAX_PLY},
};

use accumulator::Accumulator;

mod accumulator;
mod simd;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 512;

const NETWORK_SCALE: i32 = 400;
const NETWORK_QA: i32 = 384;
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

        let accumulators = &self.stack[self.index];

        let stm = accumulators.values[board.side_to_move()];
        let nstm = accumulators.values[!board.side_to_move()];

        let weights = &PARAMETERS.output_weights;

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        (output / NETWORK_QA + i32::from(PARAMETERS.output_bias)) * NETWORK_SCALE / (NETWORK_QA * NETWORK_QB)
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
