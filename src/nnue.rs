use crate::types::{Color, Piece, Square};

mod simd;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 384;
const OUTPUT_BUCKETS: usize = 4;

const EVAL_SCALE: i32 = 400;
const L0_SCALE: i32 = 256;
const L1_SCALE: i32 = 64;

#[derive(Clone)]
pub struct Network {
    accumulators: [[i16; HIDDEN_SIZE]; 2],
    stack: Vec<[[i16; HIDDEN_SIZE]; 2]>,
}

impl Network {
    /// Pushes the current state of accumulators onto the stack.
    pub fn push(&mut self) {
        self.stack.push(self.accumulators);
    }

    /// Pops the topmost state from the stack and restores the accumulators.
    pub fn pop(&mut self) {
        self.accumulators = self.stack.pop().unwrap();
    }

    /// Computes the output score for the given color.
    pub fn evaluate(&self, side_to_move: Color, piece_count: usize) -> i32 {
        let stm = self.accumulators[side_to_move];
        let nstm = self.accumulators[!side_to_move];

        let bucket = bucket(piece_count);
        let weights = &PARAMETERS.output_weights[bucket];

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        (output / L0_SCALE + i32::from(PARAMETERS.output_bias[bucket])) * EVAL_SCALE / (L0_SCALE * L1_SCALE)
    }

    /// Activates the specified piece.
    pub fn activate(&mut self, color: Color, piece: Piece, square: Square) {
        let (white, black) = index(color, piece, square);
        for i in 0..HIDDEN_SIZE {
            self.accumulators[0][i] += PARAMETERS.input_weights[white][i];
            self.accumulators[1][i] += PARAMETERS.input_weights[black][i];
        }
    }

    /// Deactivates the specified piece.
    pub fn deactivate(&mut self, color: Color, piece: Piece, square: Square) {
        let (white, black) = index(color, piece, square);
        for i in 0..HIDDEN_SIZE {
            self.accumulators[0][i] -= PARAMETERS.input_weights[white][i];
            self.accumulators[1][i] -= PARAMETERS.input_weights[black][i];
        }
    }
}

fn bucket(count: usize) -> usize {
    (count - 2) / 8
}

fn index(color: Color, piece: Piece, square: Square) -> (usize, usize) {
    (
        384 * color as usize + 64 * piece as usize + square as usize,
        384 * !color as usize + 64 * piece as usize + (square ^ 56) as usize,
    )
}

impl Default for Network {
    fn default() -> Self {
        Self {
            accumulators: [PARAMETERS.input_bias.data; 2],
            stack: Vec::default(),
        }
    }
}

#[repr(C)]
struct Parameters {
    input_weights: AlignedBlock<[[i16; HIDDEN_SIZE]; INPUT_SIZE]>,
    input_bias: AlignedBlock<[i16; HIDDEN_SIZE]>,
    output_weights: AlignedBlock<[[[i16; HIDDEN_SIZE]; 2]; OUTPUT_BUCKETS]>,
    output_bias: AlignedBlock<[i16; OUTPUT_BUCKETS]>,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

#[repr(align(64))]
struct AlignedBlock<T> {
    data: T,
}

impl<T> std::ops::Deref for AlignedBlock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
