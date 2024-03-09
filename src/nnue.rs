use crate::types::{Color, Piece, Score, Square};

mod simd;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 128;

const EVAL_SCALE: i32 = 200;
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
    pub fn evaluate(&self, side_to_move: Color) -> i32 {
        let stm = self.accumulators[side_to_move];
        let nstm = self.accumulators[!side_to_move];
        let weights = &PARAMETERS.output_weights;

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        let score = (output / L0_SCALE + i32::from(PARAMETERS.output_bias)) * EVAL_SCALE / (L0_SCALE * L1_SCALE);

        score.clamp(-Score::MATE_BOUND + 1, Score::MATE_BOUND - 1)
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

fn index(color: Color, piece: Piece, square: Square) -> (usize, usize) {
    (
        384 * color as usize + 64 * piece as usize + square as usize,
        384 * !color as usize + 64 * piece as usize + (square ^ 56) as usize,
    )
}

impl Default for Network {
    fn default() -> Self {
        Self {
            accumulators: [PARAMETERS.input_bias; 2],
            stack: Vec::default(),
        }
    }
}

#[repr(C)]
struct Parameters {
    input_weights: [[i16; HIDDEN_SIZE]; INPUT_SIZE],
    input_bias: [i16; HIDDEN_SIZE],
    output_weights: [[i16; HIDDEN_SIZE]; 2],
    output_bias: i16,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!("../networks/model.nnue")) };
