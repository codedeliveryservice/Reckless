use crate::types::{Color, Piece, Square};

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 64;

const K: i32 = 350;
const L0: i32 = 256;
const L1: i32 = 64;

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

        let mut output = i32::from(PARAMETERS.output_bias);
        for i in 0..HIDDEN_SIZE {
            output += crelu(i32::from(stm[i])) * i32::from(PARAMETERS.output_weights[0][i]);
            output += crelu(i32::from(nstm[i])) * i32::from(PARAMETERS.output_weights[1][i]);
        }
        output * K / (L0 * L1)
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

fn crelu(x: i32) -> i32 {
    x.clamp(0, L0)
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

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(r"D:\Repos\RecklessTrainer\networks\nn-24-256-64.nnue")) };
