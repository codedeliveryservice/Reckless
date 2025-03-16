use crate::{
    board::Board,
    types::{Color, Move, PieceType, Square, MAX_PLY},
};

mod simd;

const INPUT_SIZE: usize = 768;
const HIDDEN_SIZE: usize = 512;

const EVAL_SCALE: i32 = 400;
const L0_SCALE: i32 = 384;
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
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        self.stack[self.index + 1] = self.stack[self.index];
        self.index += 1;

        let piece = board.piece_on(mv.from());
        let stm = board.side_to_move();

        let add1 = index(stm, mv.promotion_piece().unwrap_or(piece.piece_type()), mv.to());
        let sub1 = index(stm, piece.piece_type(), mv.from());

        if mv.is_castling() {
            let (rook_from, root_to) = Board::get_castling_rook(mv.to());

            let add2 = index(stm, PieceType::Rook, root_to);
            let sub2 = index(stm, PieceType::Rook, rook_from);

            self.add2_sub2(add1, add2, sub1, sub2);
        } else if mv.is_capture() {
            let square = if mv.is_en_passant() { mv.to() ^ 8 } else { mv.to() };
            let piece = board.piece_on(square).piece_type();

            let sub2 = index(!stm, piece, square);

            self.add1_sub2(add1, sub1, sub2);
        } else {
            self.add1_sub1(add1, sub1);
        }
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn evaluate(&self, board: &Board) -> i32 {
        let accumulators = &self.stack[self.index];

        let stm = accumulators[board.side_to_move()];
        let nstm = accumulators[!board.side_to_move()];

        let weights = &PARAMETERS.output_weights;

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        (output / L0_SCALE + i32::from(PARAMETERS.output_bias.data)) * EVAL_SCALE / (L0_SCALE * L1_SCALE)
    }

    fn add1_sub1(&mut self, add1: FtIndex, sub1: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add1.0, i) - ft!(sub1.0, i);
            accumulators[1][i] += ft!(add1.1, i) - ft!(sub1.1, i);
        }
    }

    fn add1_sub2(&mut self, add1: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add1.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            accumulators[1][i] += ft!(add1.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }

    fn add2_sub2(&mut self, add1: FtIndex, add2: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        let accumulators = &mut self.stack[self.index];
        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] += ft!(add1.0, i) + ft!(add2.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            accumulators[1][i] += ft!(add1.1, i) + ft!(add2.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }

    pub fn refresh(&mut self, board: &Board) {
        let accumulators = &mut self.stack[self.index];

        for i in 0..HIDDEN_SIZE {
            accumulators[0][i] = PARAMETERS.input_bias[i];
            accumulators[1][i] = PARAMETERS.input_bias[i];
        }

        for square in board.occupancies() {
            let piece = board.piece_on(square);
            let (white, black) = index(piece.piece_color(), piece.piece_type(), square);

            for i in 0..HIDDEN_SIZE {
                accumulators[0][i] += ft!(white, i);
                accumulators[1][i] += ft!(black, i);
            }
        }
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
