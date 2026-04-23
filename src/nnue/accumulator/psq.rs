use super::{Aligned, L1_SIZE};
use crate::{
    board::Board,
    nnue::{AccumulatorCache, INPUT_BUCKETS_LAYOUT, Parameters, accumulator::CacheEntry, simd},
    types::{ArrayVec, Bitboard, Color, Move, MoveKind, Piece, PieceType, Square},
};

pub type PstFeature = u16;

#[derive(Clone)]
pub struct PstDelta {
    pub mv: Move,
    pub piece: Piece,
    pub captured: Piece,
}

#[derive(Clone)]
pub struct PstAccumulator {
    pub values: Aligned<[[i16; L1_SIZE]; 2]>,
    pub delta: PstDelta,
    pub accurate: [bool; 2],
}

impl PstAccumulator {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            values: Aligned::new([parameters.ft_biases.data; 2]),
            delta: PstDelta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, board: &Board, pov: Color, cache: &mut AccumulatorCache, parameters: &Parameters) {
        let king = board.king_square(pov);

        let entry = &mut cache.entries[pov][(king.is_kingside()) as usize]
            [INPUT_BUCKETS_LAYOUT[king as usize ^ (56 * pov as usize)] as usize];

        let mut adds = ArrayVec::<PstFeature, 64>::new();
        let mut subs = ArrayVec::<PstFeature, 64>::new();

        for color in [Color::White, Color::Black] {
            for piece_type in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                let pieces = board.colored_pieces(color, piece_type);
                let to_add = pieces & !(entry.pieces[piece_type] & entry.colors[color]);
                let to_sub = !pieces & (entry.pieces[piece_type] & entry.colors[color]);

                Self::push_features(&mut adds, color, piece_type, to_add, king, pov);
                Self::push_features(&mut subs, color, piece_type, to_sub, king, pov);
            }
        }

        unsafe { apply_changes(entry, adds, subs, parameters) };

        entry.pieces = board.pieces_bbs();
        entry.colors = board.colors_bbs();

        self.values[pov] = *entry.values;
        self.accurate[pov] = true;
    }

    #[inline]
    #[cfg(not(target_feature = "avx512vbmi2"))]
    fn push_features(
        features: &mut ArrayVec<PstFeature, 64>, color: Color, piece_type: PieceType, bb: Bitboard, king: Square,
        pov: Color,
    ) {
        for square in bb {
            features.push(pst_index(color, piece_type, square, king, pov));
        }
    }

    #[inline]
    #[cfg(target_feature = "avx512vbmi2")]
    fn push_features(
        features: &mut ArrayVec<PstFeature, 64>, color: Color, piece_type: PieceType, bb: Bitboard, king: Square,
        pov: Color,
    ) {
        unsafe {
            use std::arch::x86_64::*;

            let base = pst_index(color, piece_type, Square::new(0), king, pov);

            let iota = _mm512_set_epi8(
                63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39, 38,
                37, 36, 35, 34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12,
                11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
            );
            let squares = _mm512_castsi512_si128(_mm512_maskz_compress_epi8(bb.0, iota));
            let to_write = _mm256_xor_si256(_mm256_set1_epi16(base as i16), _mm256_cvtepu8_epi16(squares));
            features.unchecked_write(|data| {
                _mm256_storeu_si256(data.cast(), to_write);
                bb.count()
            });
        }
    }

    pub fn update(&mut self, prev: &Self, board: &Board, king: Square, pov: Color, parameters: &Parameters) {
        let PstDelta { mv, piece, captured } = self.delta;

        let resulting_piece = if mv.is_promotion() { mv.promo_piece_type() } else { piece.piece_type() };

        let add1 = pst_index(piece.color(), resulting_piece, mv.to(), king, pov);
        let sub1 = pst_index(piece.color(), piece.piece_type(), mv.from(), king, pov);

        match mv.kind() {
            MoveKind::Castling => {
                let (rook_from, rook_to) = board.get_castling_rook(mv.to());

                let add2 = pst_index(piece.color(), PieceType::Rook, rook_to, king, pov);
                let sub2 = pst_index(piece.color(), PieceType::Rook, rook_from, king, pov);

                self.add2_sub2(prev, add1, add2, sub1, sub2, pov, parameters);
            }
            MoveKind::EnPassant => {
                let sub2 = pst_index(!piece.color(), PieceType::Pawn, mv.to() ^ 8, king, pov);
                self.add1_sub2(prev, add1, sub1, sub2, pov, parameters);
            }
            MoveKind::Capture
            | MoveKind::PromotionCaptureN
            | MoveKind::PromotionCaptureB
            | MoveKind::PromotionCaptureR
            | MoveKind::PromotionCaptureQ => {
                let sub2 = pst_index(!piece.color(), captured.piece_type(), mv.to(), king, pov);
                self.add1_sub2(prev, add1, sub1, sub2, pov, parameters);
            }
            _ => self.add1_sub1(prev, add1, sub1, pov, parameters),
        }

        self.accurate[pov] = true;
    }

    fn add1_sub1(&mut self, prev: &Self, add1: PstFeature, sub1: PstFeature, pov: Color, parameters: &Parameters) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = parameters.ft_piece_weights[add1 as usize].as_ptr();
        let vsub1 = parameters.ft_piece_weights[sub1 as usize].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, simd::sub_i16(*vadd1.add(i).cast(), *vsub1.add(i).cast()));

                *vacc.add(i).cast() = v;
            }
        }
    }

    fn add1_sub2(
        &mut self, prev: &Self, add1: PstFeature, sub1: PstFeature, sub2: PstFeature, pov: Color,
        parameters: &Parameters,
    ) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = parameters.ft_piece_weights[add1 as usize].as_ptr();
        let vsub1 = parameters.ft_piece_weights[sub1 as usize].as_ptr();
        let vsub2 = parameters.ft_piece_weights[sub2 as usize].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, *vadd1.add(i).cast());
                v = simd::sub_i16(v, simd::add_i16(*vsub1.add(i).cast(), *vsub2.add(i).cast()));

                *vacc.add(i).cast() = v;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add2_sub2(
        &mut self, prev: &Self, add1: PstFeature, add2: PstFeature, sub1: PstFeature, sub2: PstFeature, pov: Color,
        parameters: &Parameters,
    ) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = parameters.ft_piece_weights[add1 as usize].as_ptr();
        let vadd2 = parameters.ft_piece_weights[add2 as usize].as_ptr();
        let vsub1 = parameters.ft_piece_weights[sub1 as usize].as_ptr();
        let vsub2 = parameters.ft_piece_weights[sub2 as usize].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, simd::add_i16(*vadd1.add(i).cast(), *vadd2.add(i).cast()));
                v = simd::sub_i16(v, simd::add_i16(*vsub1.add(i).cast(), *vsub2.add(i).cast()));

                *vacc.add(i).cast() = v;
            }
        }
    }
}

const REGISTERS: usize = 8;
const _: () = assert!(L1_SIZE.is_multiple_of(REGISTERS * simd::I16_LANES));

unsafe fn apply_changes(
    entry: &mut CacheEntry, adds: ArrayVec<PstFeature, 64>, subs: ArrayVec<PstFeature, 64>, parameters: &Parameters,
) {
    let mut registers: [_; REGISTERS] = std::mem::zeroed();

    for offset in (0..L1_SIZE).step_by(REGISTERS * simd::I16_LANES) {
        let output = entry.values.as_mut_ptr().add(offset);

        for (i, register) in registers.iter_mut().enumerate() {
            *register = *output.add(i * simd::I16_LANES).cast();
        }

        for &add in adds.iter() {
            let weights = parameters.ft_piece_weights[add as usize].as_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = simd::add_i16(*register, *weights.add(i * simd::I16_LANES).cast());
            }
        }

        for &sub in subs.iter() {
            let weights = parameters.ft_piece_weights[sub as usize].as_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = simd::sub_i16(*register, *weights.add(i * simd::I16_LANES).cast());
            }
        }

        for (i, register) in registers.into_iter().enumerate() {
            *output.add(i * simd::I16_LANES).cast() = register;
        }
    }
}

fn pst_index(color: Color, piece: PieceType, square: Square, king: Square, pov: Color) -> PstFeature {
    let flip = (7 * ((king.is_kingside()) as u8)) ^ (56 * (pov as u8));

    INPUT_BUCKETS_LAYOUT[king ^ flip] as PstFeature * 768
        + 384 * (color != pov) as PstFeature
        + 64 * piece as PstFeature
        + (square ^ flip) as PstFeature
}
