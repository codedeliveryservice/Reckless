use crate::{
    board::Board,
    nnue::{
        Aligned, INPUT_BUCKETS_LAYOUT, L1_SIZE, PARAMETERS,
        accumulator::{AccumulatorCache, CacheEntry},
        simd,
    },
    types::{ArrayVec, Color, Move, MoveKind, Piece, PieceType, Square},
};

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
    pub fn new() -> Self {
        Self {
            values: Aligned::new([PARAMETERS.ft_biases.data; 2]),
            delta: PstDelta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, board: &Board, pov: Color, cache: &mut AccumulatorCache) {
        let king = board.king_square(pov);

        let entry = &mut cache.entries[pov][(king.is_kingside()) as usize]
            [INPUT_BUCKETS_LAYOUT[king as usize ^ (56 * pov as usize)]];

        let mut adds = ArrayVec::<_, 32>::new();
        let mut subs = ArrayVec::<_, 32>::new();

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
                let to_add = pieces & !(entry.pieces[piece_type] & entry.colors[color]);
                let to_sub = !pieces & (entry.pieces[piece_type] & entry.colors[color]);

                for square in to_add {
                    adds.push(pst_index(color, piece_type, square, king, pov));
                }

                for square in to_sub {
                    subs.push(pst_index(color, piece_type, square, king, pov));
                }
            }
        }

        unsafe { apply_changes(entry, adds, subs) };

        entry.pieces = board.pieces_bbs();
        entry.colors = board.colors_bbs();

        self.values[pov] = *entry.values;
        self.accurate[pov] = true;
    }

    pub fn update(&mut self, prev: &Self, board: &Board, king: Square, pov: Color) {
        let PstDelta { mv, piece, captured } = self.delta;

        let resulting_piece = mv.promotion_piece().unwrap_or_else(|| piece.piece_type());

        let add1 = pst_index(piece.piece_color(), resulting_piece, mv.to(), king, pov);
        let sub1 = pst_index(piece.piece_color(), piece.piece_type(), mv.from(), king, pov);

        match mv.kind() {
            MoveKind::Castling => {
                let (rook_from, rook_to) = board.get_castling_rook(mv.to());

                let add2 = pst_index(piece.piece_color(), PieceType::Rook, rook_to, king, pov);
                let sub2 = pst_index(piece.piece_color(), PieceType::Rook, rook_from, king, pov);

                self.add2_sub2(prev, add1, add2, sub1, sub2, pov);
            }
            MoveKind::EnPassant => {
                let sub2 = pst_index(!piece.piece_color(), PieceType::Pawn, mv.to() ^ 8, king, pov);
                self.add1_sub2(prev, add1, sub1, sub2, pov);
            }
            MoveKind::Capture
            | MoveKind::PromotionCaptureN
            | MoveKind::PromotionCaptureB
            | MoveKind::PromotionCaptureR
            | MoveKind::PromotionCaptureQ => {
                let sub2 = pst_index(!piece.piece_color(), captured.piece_type(), mv.to(), king, pov);
                self.add1_sub2(prev, add1, sub1, sub2, pov);
            }
            _ => self.add1_sub1(prev, add1, sub1, pov),
        }

        self.accurate[pov] = true;
    }

    fn add1_sub1(&mut self, prev: &Self, add1: usize, sub1: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = PARAMETERS.ft_piece_weights[add1].as_ptr();
        let vsub1 = PARAMETERS.ft_piece_weights[sub1].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, *vadd1.add(i).cast());
                v = simd::sub_i16(v, *vsub1.add(i).cast());

                *vacc.add(i).cast() = v;
            }
        }
    }

    fn add1_sub2(&mut self, prev: &Self, add1: usize, sub1: usize, sub2: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = PARAMETERS.ft_piece_weights[add1].as_ptr();
        let vsub1 = PARAMETERS.ft_piece_weights[sub1].as_ptr();
        let vsub2 = PARAMETERS.ft_piece_weights[sub2].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, *vadd1.add(i).cast());
                v = simd::sub_i16(v, *vsub1.add(i).cast());
                v = simd::sub_i16(v, *vsub2.add(i).cast());

                *vacc.add(i).cast() = v;
            }
        }
    }

    fn add2_sub2(&mut self, prev: &Self, add1: usize, add2: usize, sub1: usize, sub2: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = PARAMETERS.ft_piece_weights[add1].as_ptr();
        let vadd2 = PARAMETERS.ft_piece_weights[add2].as_ptr();
        let vsub1 = PARAMETERS.ft_piece_weights[sub1].as_ptr();
        let vsub2 = PARAMETERS.ft_piece_weights[sub2].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, *vadd1.add(i).cast());
                v = simd::add_i16(v, *vadd2.add(i).cast());
                v = simd::sub_i16(v, *vsub1.add(i).cast());
                v = simd::sub_i16(v, *vsub2.add(i).cast());

                *vacc.add(i).cast() = v;
            }
        }
    }
}

const REGISTERS: usize = 8;
const _: () = assert!(L1_SIZE % (REGISTERS * simd::I16_LANES) == 0);

unsafe fn apply_changes(entry: &mut CacheEntry, adds: ArrayVec<usize, 32>, subs: ArrayVec<usize, 32>) {
    let mut registers: [_; REGISTERS] = std::mem::zeroed();

    for offset in (0..L1_SIZE).step_by(REGISTERS * simd::I16_LANES) {
        let output = entry.values.as_mut_ptr().add(offset);

        for (i, register) in registers.iter_mut().enumerate() {
            *register = *output.add(i * simd::I16_LANES).cast();
        }

        for &add in adds.iter() {
            let weights = PARAMETERS.ft_piece_weights[add].as_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = simd::add_i16(*register, *weights.add(i * simd::I16_LANES).cast());
            }
        }

        for &sub in subs.iter() {
            let weights = PARAMETERS.ft_piece_weights[sub].as_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = simd::sub_i16(*register, *weights.add(i * simd::I16_LANES).cast());
            }
        }

        for (i, register) in registers.into_iter().enumerate() {
            *output.add(i * simd::I16_LANES).cast() = register;
        }
    }
}

fn pst_index(color: Color, piece: PieceType, square: Square, king: Square, pov: Color) -> usize {
    let flip = (7 * ((king.is_kingside()) as u8)) ^ (56 * (pov as u8));

    INPUT_BUCKETS_LAYOUT[king ^ flip] * 768
        + 384 * (color != pov) as usize
        + 64 * piece as usize
        + (square ^ flip) as usize
}
