use super::{simd, Aligned, BUCKETS, HIDDEN_SIZE, INPUT_BUCKETS, INPUT_SIZE, PARAMETERS};
use crate::{
    board::Board,
    types::{Bitboard, Color, Move, Piece, PieceType, Square},
};

#[derive(Clone, Default)]
pub struct AccumulatorCache {
    entries: Box<[[[CacheEntry; INPUT_BUCKETS]; 2]; 2]>,
}

#[derive(Clone)]
pub struct CacheEntry {
    accumulator: Aligned<[f32; HIDDEN_SIZE]>,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
}

impl Default for CacheEntry {
    fn default() -> Self {
        Self {
            accumulator: PARAMETERS.ft_biases,
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
        }
    }
}

#[derive(Copy, Clone)]
pub struct Delta {
    pub mv: Move,
    pub piece: Piece,
    pub captured: Piece,
}

#[derive(Copy, Clone)]
pub struct Accumulator {
    pub values: Aligned<[[f32; HIDDEN_SIZE]; 2]>,
    pub delta: Delta,
    pub accurate: [bool; 2],
}

impl Accumulator {
    pub fn new() -> Self {
        Self {
            values: Aligned { data: [PARAMETERS.ft_biases.data; 2] },
            delta: Delta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, board: &Board, pov: Color, cache: &mut AccumulatorCache) {
        let king = board.king_square(pov);

        let entry = &mut cache.entries[pov][(king.file() >= 4) as usize]
            [BUCKETS[if pov == Color::White { king } else { king ^ 56 }]];

        for color in [Color::White, Color::Black] {
            for piece_type in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                let pieces = board.of(piece_type, color);
                let adds = pieces & !(entry.pieces[piece_type] & entry.colors[color]);
                let subs = !pieces & (entry.pieces[piece_type] & entry.colors[color]);

                for square in adds {
                    let feature = index(color, piece_type, square, king, pov);

                    for i in 0..HIDDEN_SIZE {
                        entry.accumulator[i] += PARAMETERS.ft_weights[feature][i];
                    }
                }

                for square in subs {
                    let feature = index(color, piece_type, square, king, pov);

                    for i in 0..HIDDEN_SIZE {
                        entry.accumulator[i] -= PARAMETERS.ft_weights[feature][i];
                    }
                }
            }
        }

        entry.pieces = board.pieces_bbs();
        entry.colors = board.colors_bbs();

        self.values[pov] = *entry.accumulator;
        self.accurate[pov] = true;
    }

    pub fn update(&mut self, prev: &Self, king: Square, pov: Color) {
        let Delta { mv, piece, captured } = self.delta;

        let resulting_piece = mv.promotion_piece().unwrap_or_else(|| piece.piece_type());

        let add1 = index(piece.piece_color(), resulting_piece, mv.to(), king, pov);
        let sub1 = index(piece.piece_color(), piece.piece_type(), mv.from(), king, pov);

        if mv.is_castling() {
            let (rook_from, root_to) = Board::get_castling_rook(mv.to());

            let add2 = index(piece.piece_color(), PieceType::Rook, root_to, king, pov);
            let sub2 = index(piece.piece_color(), PieceType::Rook, rook_from, king, pov);

            self.add2_sub2(prev, add1, add2, sub1, sub2, pov);
        } else if mv.is_capture() {
            let sub2 = if mv.is_en_passant() {
                index(!piece.piece_color(), PieceType::Pawn, mv.to() ^ 8, king, pov)
            } else {
                index(!piece.piece_color(), captured.piece_type(), mv.to(), king, pov)
            };

            self.add1_sub2(prev, add1, sub1, sub2, pov);
        } else {
            self.add1_sub1(prev, add1, sub1, pov);
        }

        self.accurate[pov] = true;
    }

    fn add1_sub1(&mut self, prev: &Self, add1: usize, sub1: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr().cast::<simd::Vector>();
        let vprev = prev.values[pov].as_ptr().cast::<simd::Vector>();

        let vadd1 = PARAMETERS.ft_weights[add1].as_ptr().cast::<simd::Vector>();
        let vsub1 = PARAMETERS.ft_weights[sub1].as_ptr().cast::<simd::Vector>();

        for i in 0..HIDDEN_SIZE / simd::VECTOR_WIDTH {
            unsafe {
                let mut v = *vprev.add(i);
                v = simd::add(v, *vadd1.add(i));
                v = simd::sub(v, *vsub1.add(i));

                *vacc.add(i) = v;
            }
        }
    }

    fn add1_sub2(&mut self, prev: &Self, add1: usize, sub1: usize, sub2: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr().cast::<simd::Vector>();
        let vprev = prev.values[pov].as_ptr().cast::<simd::Vector>();

        let vadd1 = PARAMETERS.ft_weights[add1].as_ptr().cast::<simd::Vector>();
        let vsub1 = PARAMETERS.ft_weights[sub1].as_ptr().cast::<simd::Vector>();
        let vsub2 = PARAMETERS.ft_weights[sub2].as_ptr().cast::<simd::Vector>();

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

    fn add2_sub2(&mut self, prev: &Self, add1: usize, add2: usize, sub1: usize, sub2: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr().cast::<simd::Vector>();
        let vprev = prev.values[pov].as_ptr().cast::<simd::Vector>();

        let vadd1 = PARAMETERS.ft_weights[add1].as_ptr().cast::<simd::Vector>();
        let vadd2 = PARAMETERS.ft_weights[add2].as_ptr().cast::<simd::Vector>();
        let vsub1 = PARAMETERS.ft_weights[sub1].as_ptr().cast::<simd::Vector>();
        let vsub2 = PARAMETERS.ft_weights[sub2].as_ptr().cast::<simd::Vector>();

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

fn index(color: Color, piece: PieceType, mut square: Square, mut king: Square, pov: Color) -> usize {
    if king.file() >= 4 {
        square ^= 7;
        king ^= 7;
    }

    if pov == Color::Black {
        square ^= 56;
        king ^= 56;
    }

    BUCKETS[king] * INPUT_SIZE + 384 * (color != pov) as usize + 64 * piece as usize + square as usize
}
