use crate::types::{Color, Piece, PieceType, Square, MAX_PLY};

mod simd;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 384;

const EVAL_SCALE: i32 = 400;
const L0_SCALE: i32 = 256;
const L1_SCALE: i32 = 64;

type FtIndex = (usize, usize);

macro_rules! ft {
    ($feature:expr, $i:expr) => {
        PARAMETERS.input_weights[$feature][$i]
    };
}

#[derive(Clone)]
pub struct Network {
    index: usize,
    stack: Box<[[[i16; HIDDEN_SIZE]; 2]; MAX_PLY]>,
    adds: Vec<FtIndex>,
    subs: Vec<FtIndex>,
}

impl Network {
    /// Pushes the current state of accumulators onto the stack.
    pub fn push(&mut self) {
        self.stack[self.index + 1] = self.stack[self.index];
        self.index += 1;
    }

    /// Pops the topmost state from the stack and restores the accumulators.
    pub fn pop(&mut self) {
        self.index -= 1;
    }

    /// Computes the output score for the given color.
    pub fn evaluate(&self, side_to_move: Color) -> i32 {
        let accumulators = &self.stack[self.index];

        let stm = accumulators[side_to_move];
        let nstm = accumulators[!side_to_move];

        let weights = &PARAMETERS.output_weights;

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        (output / L0_SCALE + i32::from(PARAMETERS.output_bias.data)) * EVAL_SCALE / (L0_SCALE * L1_SCALE)
    }

    pub fn commit(&mut self) {
        match (&self.adds[..], &self.subs[..]) {
            (&[add], &[sub]) => self.add1_sub1(add, sub),
            (&[add], &[sub1, sub2]) => self.add1_sub2(add, sub1, sub2),
            (&[add1, add2], &[sub1, sub2]) => self.add2_sub2(add1, add2, sub1, sub2),
            (&[add1, add2], &[sub1, sub2, _]) => self.add2_sub2(add1, add2, sub1, sub2),
            _ => panic!(),
        }

        self.clear_buffers();
    }

    pub fn clear_buffers(&mut self) {
        self.adds.clear();
        self.subs.clear();
    }

    fn add1_sub1(&mut self, add: FtIndex, sub: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add.0, i) - ft!(sub.0, i);
            accumulators[1][i] += ft!(add.1, i) - ft!(sub.1, i);
        }
    }

    fn add1_sub2(&mut self, add: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            accumulators[1][i] += ft!(add.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }

    fn add2_sub2(&mut self, add1: FtIndex, add2: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add1.0, i) + ft!(add2.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            accumulators[1][i] += ft!(add1.1, i) + ft!(add2.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }

    pub fn accumulate(&mut self, piece: Piece, square: Square) {
        let (white, black) = index(piece.piece_color(), piece.piece_type(), square);
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += PARAMETERS.input_weights[white][i];
            accumulators[1][i] += PARAMETERS.input_weights[black][i];
        }
    }

    pub fn activate(&mut self, piece: Piece, square: Square) {
        self.adds.push(index(piece.piece_color(), piece.piece_type(), square));
    }

    pub fn deactivate(&mut self, piece: Piece, square: Square) {
        self.subs.push(index(piece.piece_color(), piece.piece_type(), square));
    }
}

fn index(color: Color, piece: PieceType, square: Square) -> FtIndex {
    (
        384 * color as usize + 64 * piece as usize + square as usize,
        384 * !color as usize + 64 * piece as usize + (square ^ 56) as usize,
    )
}

impl Default for Network {
    fn default() -> Self {
        Self {
            index: 0,
            stack: Box::new([[PARAMETERS.input_bias.data; 2]; MAX_PLY]),
            adds: Vec::default(),
            subs: Vec::default(),
        }
    }
}

#[repr(C)]
struct Parameters {
    input_weights: AlignedBlock<[[i16; HIDDEN_SIZE]; INPUT_SIZE]>,
    input_bias: AlignedBlock<[i16; HIDDEN_SIZE]>,
    output_weights: AlignedBlock<[[i16; HIDDEN_SIZE]; 2]>,
    output_bias: AlignedBlock<i16>,
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
