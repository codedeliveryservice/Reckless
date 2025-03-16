use crate::{
    board::Board,
    types::{Color, Move, Piece, PieceType, Square, MAX_PLY},
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
    stack: Box<[Accumulator; MAX_PLY]>,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv != Move::NULL);

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
            let index = (0..self.index).rfind(|&i| self.stack[i].accurate).unwrap();

            for i in index..self.index {
                if let (prev, [current, ..]) = self.stack.split_at_mut(i + 1) {
                    current.update(&prev[i]);
                }
            }
        }

        let accumulators = &self.stack[self.index];

        let stm = accumulators.values[board.side_to_move()];
        let nstm = accumulators.values[!board.side_to_move()];

        let weights = &PARAMETERS.output_weights;

        let output = simd::forward(&stm, &weights[0]) + simd::forward(&nstm, &weights[1]);
        (output / L0_SCALE + i32::from(PARAMETERS.output_bias.data)) * EVAL_SCALE / (L0_SCALE * L1_SCALE)
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
        Self { index: 0, stack: Box::new([Accumulator::new(); MAX_PLY]) }
    }
}

#[repr(C)]
struct Parameters {
    input_weights: Aligned<[[i16; HIDDEN_SIZE]; INPUT_SIZE]>,
    input_bias: Aligned<[i16; HIDDEN_SIZE]>,
    output_weights: Aligned<[[i16; HIDDEN_SIZE]; 2]>,
    output_bias: Aligned<i16>,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

#[derive(Clone, Copy)]
struct Delta {
    mv: Move,
    piece: Piece,
    captured: Piece,
}

#[derive(Clone, Copy)]
struct Accumulator {
    values: Aligned<[[i16; HIDDEN_SIZE]; 2]>,
    delta: Delta,
    accurate: bool,
}

impl Accumulator {
    pub fn new() -> Self {
        Self {
            values: Aligned { data: [PARAMETERS.input_bias.data; 2] },
            delta: Delta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: false,
        }
    }

    pub fn refresh(&mut self, board: &Board) {
        for i in 0..HIDDEN_SIZE {
            self.values[0][i] = PARAMETERS.input_bias[i];
            self.values[1][i] = PARAMETERS.input_bias[i];
        }

        for square in board.occupancies() {
            let piece = board.piece_on(square);
            let (white, black) = index(piece.piece_color(), piece.piece_type(), square);

            for i in 0..HIDDEN_SIZE {
                self.values[0][i] += ft!(white, i);
                self.values[1][i] += ft!(black, i);
            }
        }

        self.accurate = true;
    }

    pub fn update(&mut self, prev: &Accumulator) {
        let Delta { mv, piece, captured } = self.delta;

        let add1 = index(piece.piece_color(), mv.promotion_piece().unwrap_or(piece.piece_type()), mv.to());
        let sub1 = index(piece.piece_color(), piece.piece_type(), mv.from());

        if mv.is_castling() {
            let (rook_from, root_to) = Board::get_castling_rook(mv.to());

            let add2 = index(piece.piece_color(), PieceType::Rook, root_to);
            let sub2 = index(piece.piece_color(), PieceType::Rook, rook_from);

            self.add2_sub2(prev, add1, add2, sub1, sub2);
        } else if mv.is_capture() {
            let mut square = mv.to();
            let mut captured = captured.piece_type();

            if mv.is_en_passant() {
                square = square ^ 8;
                captured = PieceType::Pawn;
            }

            let sub2 = index(!piece.piece_color(), captured, square);

            self.add1_sub2(prev, add1, sub1, sub2);
        } else {
            self.add1_sub1(prev, add1, sub1);
        }

        self.accurate = true;
    }

    fn add1_sub1(&mut self, prev: &Accumulator, add1: FtIndex, sub1: FtIndex) {
        for i in 0..HIDDEN_SIZE {
            self.values[0][i] = prev.values[0][i] + ft!(add1.0, i) - ft!(sub1.0, i);
            self.values[1][i] = prev.values[1][i] + ft!(add1.1, i) - ft!(sub1.1, i);
        }
    }

    fn add1_sub2(&mut self, prev: &Accumulator, add1: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        for i in 0..HIDDEN_SIZE {
            self.values[0][i] = prev.values[0][i] + ft!(add1.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            self.values[1][i] = prev.values[1][i] + ft!(add1.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }

    fn add2_sub2(&mut self, prev: &Accumulator, add1: FtIndex, add2: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        for i in 0..HIDDEN_SIZE {
            self.values[0][i] = prev.values[0][i] + ft!(add1.0, i) + ft!(add2.0, i) - ft!(sub1.0, i) - ft!(sub2.0, i);
            self.values[1][i] = prev.values[1][i] + ft!(add1.1, i) + ft!(add2.1, i) - ft!(sub1.1, i) - ft!(sub2.1, i);
        }
    }
}

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
