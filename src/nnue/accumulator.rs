use super::{simd, Aligned, HIDDEN_SIZE, PARAMETERS};
use crate::{
    board::Board,
    types::{Color, Move, Piece, PieceType, Square},
};

type FtIndex = [usize; 2];

macro_rules! ft {
    ($feature:expr, $i:expr) => {
        PARAMETERS.ft_weights[$feature][$i]
    };
}

#[derive(Copy, Clone)]
pub struct Delta {
    pub mv: Move,
    pub piece: Piece,
    pub captured: Piece,
}

#[derive(Copy, Clone)]
pub struct Accumulator {
    pub values: Aligned<[[i16; HIDDEN_SIZE]; 2]>,
    pub delta: Delta,
    pub accurate: bool,
}

impl Accumulator {
    pub fn new() -> Self {
        Self {
            values: Aligned { data: [PARAMETERS.ft_biases.data; 2] },
            delta: Delta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: false,
        }
    }

    pub fn refresh(&mut self, board: &Board) {
        let wking = board.king_square(Color::White);
        let bking = board.king_square(Color::Black);

        for i in 0..HIDDEN_SIZE {
            self.values[0][i] = PARAMETERS.ft_biases[i];
            self.values[1][i] = PARAMETERS.ft_biases[i];
        }

        for square in board.occupancies() {
            let piece = board.piece_on(square);
            let [white, black] = index(piece.piece_color(), piece.piece_type(), square, wking, bking);

            for i in 0..HIDDEN_SIZE {
                self.values[0][i] += ft!(white, i);
                self.values[1][i] += ft!(black, i);
            }
        }

        self.accurate = true;
    }

    pub fn update(&mut self, prev: &Self, wking: Square, bking: Square) {
        let Delta { mv, piece, captured } = self.delta;

        let resulting_piece = mv.promotion_piece().unwrap_or_else(|| piece.piece_type());

        let add1 = index(piece.piece_color(), resulting_piece, mv.to(), wking, bking);
        let sub1 = index(piece.piece_color(), piece.piece_type(), mv.from(), wking, bking);

        if mv.is_castling() {
            let (rook_from, root_to) = Board::get_castling_rook(mv.to());

            let add2 = index(piece.piece_color(), PieceType::Rook, root_to, wking, bking);
            let sub2 = index(piece.piece_color(), PieceType::Rook, rook_from, wking, bking);

            self.add2_sub2(prev, add1, add2, sub1, sub2);
        } else if mv.is_capture() {
            let sub2 = if mv.is_en_passant() {
                index(!piece.piece_color(), PieceType::Pawn, mv.to() ^ 8, wking, bking)
            } else {
                index(!piece.piece_color(), captured.piece_type(), mv.to(), wking, bking)
            };

            self.add1_sub2(prev, add1, sub1, sub2);
        } else {
            self.add1_sub1(prev, add1, sub1);
        }

        self.accurate = true;
    }

    fn add1_sub1(&mut self, prev: &Self, add1: FtIndex, sub1: FtIndex) {
        for side in [0, 1] {
            let vacc = self.values[side].as_mut_ptr().cast::<simd::Vector>();
            let vprev = prev.values[side].as_ptr().cast::<simd::Vector>();

            let vadd1 = PARAMETERS.ft_weights[add1[side]].as_ptr().cast::<simd::Vector>();
            let vsub1 = PARAMETERS.ft_weights[sub1[side]].as_ptr().cast::<simd::Vector>();

            for i in 0..HIDDEN_SIZE / simd::VECTOR_WIDTH {
                unsafe {
                    let mut v = *vprev.add(i);
                    v = simd::add(v, *vadd1.add(i));
                    v = simd::sub(v, *vsub1.add(i));

                    *vacc.add(i) = v;
                }
            }
        }
    }

    fn add1_sub2(&mut self, prev: &Self, add1: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        for side in [0, 1] {
            let vacc = self.values[side].as_mut_ptr().cast::<simd::Vector>();
            let vprev = prev.values[side].as_ptr().cast::<simd::Vector>();

            let vadd1 = PARAMETERS.ft_weights[add1[side]].as_ptr().cast::<simd::Vector>();
            let vsub1 = PARAMETERS.ft_weights[sub1[side]].as_ptr().cast::<simd::Vector>();
            let vsub2 = PARAMETERS.ft_weights[sub2[side]].as_ptr().cast::<simd::Vector>();

            for i in 0..HIDDEN_SIZE / simd::VECTOR_WIDTH {
                unsafe {
                    let mut v = *vprev.add(i);
                    v = simd::add(v, *vadd1.add(i));
                    v = simd::sub(v, *vsub1.add(i));
                    v = simd::sub(v, *vsub2.add(i));

                    *vacc.add(i) = v;
                }
            }
        }
    }

    fn add2_sub2(&mut self, prev: &Self, add1: FtIndex, add2: FtIndex, sub1: FtIndex, sub2: FtIndex) {
        for side in [0, 1] {
            let vacc = self.values[side].as_mut_ptr().cast::<simd::Vector>();
            let vprev = prev.values[side].as_ptr().cast::<simd::Vector>();

            let vadd1 = PARAMETERS.ft_weights[add1[side]].as_ptr().cast::<simd::Vector>();
            let vadd2 = PARAMETERS.ft_weights[add2[side]].as_ptr().cast::<simd::Vector>();
            let vsub1 = PARAMETERS.ft_weights[sub1[side]].as_ptr().cast::<simd::Vector>();
            let vsub2 = PARAMETERS.ft_weights[sub2[side]].as_ptr().cast::<simd::Vector>();

            for i in 0..HIDDEN_SIZE / simd::VECTOR_WIDTH {
                unsafe {
                    let mut v = *vprev.add(i);
                    v = simd::add(v, *vadd1.add(i));
                    v = simd::add(v, *vadd2.add(i));
                    v = simd::sub(v, *vsub1.add(i));
                    v = simd::sub(v, *vsub2.add(i));

                    *vacc.add(i) = v;
                }
            }
        }
    }
}

fn index(color: Color, piece: PieceType, square: Square, wking: Square, bking: Square) -> FtIndex {
    let wsquare = if wking.file() >= 4 { square ^ 7 } else { square };
    let bsquare = if bking.file() >= 4 { square ^ 7 } else { square };

    let white = 384 * color as usize + 64 * piece as usize + wsquare as usize;
    let black = 384 * !color as usize + 64 * piece as usize + (bsquare ^ 56) as usize;

    [white, black]
}
