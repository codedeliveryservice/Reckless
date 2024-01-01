use crate::types::{Color, Piece, Square};

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 32;

const K: i32 = 400;
const L0: i32 = 256;
const L1: i32 = 64;

#[derive(Clone)]
pub struct Network {
    accumulator: [i16; HIDDEN_SIZE],
    stack: Vec<[i16; HIDDEN_SIZE]>,
}

impl Network {
    /// Pushes the current state of accumulators onto the stack.
    pub fn push(&mut self) {
        self.stack.push(self.accumulator);
    }

    /// Pops the topmost state from the stack and restores the accumulators.
    pub fn pop(&mut self) {
        self.accumulator = self.stack.pop().unwrap();
    }

    /// Computes the output score for the given color.
    pub fn evaluate(&self) -> i32 {
        let mut output = i32::from(PARAMETERS.output_bias);
        for i in 0..HIDDEN_SIZE {
            output += i32::from(relu(self.accumulator[i])) * i32::from(PARAMETERS.output_weights[i]);
        }
        output * K / (L0 * L1)
    }

    /// Activates the specified piece.
    pub fn activate(&mut self, color: Color, piece: Piece, square: Square) {
        let index = index(color, piece, square);
        for i in 0..HIDDEN_SIZE {
            self.accumulator[i] += PARAMETERS.input_weights[index][i];
        }
    }

    /// Deactivates the specified piece.
    pub fn deactivate(&mut self, color: Color, piece: Piece, square: Square) {
        let index = index(color, piece, square);
        for i in 0..HIDDEN_SIZE {
            self.accumulator[i] -= PARAMETERS.input_weights[index][i];
        }
    }
}

fn index(color: Color, piece: Piece, square: Square) -> usize {
    384 * color as usize + 64 * piece as usize + square as usize
}

fn relu(x: i16) -> i16 {
    x.max(0)
}

impl Default for Network {
    fn default() -> Self {
        Self {
            accumulator: PARAMETERS.input_bias,
            stack: Vec::default(),
        }
    }
}

struct Parameters {
    input_weights: [[i16; HIDDEN_SIZE]; INPUT_SIZE],
    input_bias: [i16; HIDDEN_SIZE],
    output_weights: [i16; HIDDEN_SIZE],
    output_bias: i16,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(r"D:\Repos\RecklessTrainer\networks\nn-07-quantized-256-64.nnue")) };
