use crate::types::{Color, Piece, Square};

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 32;
const K: f32 = 400.0;

#[derive(Clone)]
pub struct Network {
    accumulator: [f32; HIDDEN_SIZE],
    stack: Vec<[f32; HIDDEN_SIZE]>,
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
        let mut output = PARAMETERS.output_bias;
        for i in 0..HIDDEN_SIZE {
            output += relu(self.accumulator[i]) * PARAMETERS.output_weights[i];
        }
        (output * K) as i32
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

fn relu(x: f32) -> f32 {
    x.max(0.0)
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
    input_weights: [[f32; HIDDEN_SIZE]; INPUT_SIZE],
    input_bias: [f32; HIDDEN_SIZE],
    output_weights: [f32; HIDDEN_SIZE],
    output_bias: f32,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(r"D:\Repos\RecklessTrainer\networks\random.nnue")) };
