use super::{simd, Aligned, Parameters, L1_SIZE};
use crate::{
    board::Board,
    lookup::attacks,
    nnue::{threats::threat_index, BUCKETS, INPUT_BUCKETS},
    types::{ArrayVec, Bitboard, Color, Move, Piece, PieceType, Square},
};

#[derive(Clone)]
pub struct AccumulatorCache {
    entries: Box<[[[CacheEntry; INPUT_BUCKETS]; 2]; 2]>,
}

impl AccumulatorCache {
    pub fn new(params: &Parameters) -> Self {
        let entry = CacheEntry::new(params);
        Self { entries: Box::new([[[entry; INPUT_BUCKETS]; 2]; 2]) }
    }
}

#[derive(Copy, Clone)]
pub struct CacheEntry {
    values: Aligned<[i16; L1_SIZE]>,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
}

impl CacheEntry {
    pub fn new(params: &Parameters) -> Self {
        Self {
            values: params.ft_biases,
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
        }
    }
}

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
    pub fn new(params: &Parameters) -> Self {
        Self {
            values: Aligned::new([params.ft_biases.data; 2]),
            delta: PstDelta { mv: Move::NULL, piece: Piece::None, captured: Piece::None },
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, params: &Parameters, board: &Board, pov: Color, cache: &mut AccumulatorCache) {
        let king = board.king_square(pov);

        let entry = &mut cache.entries[pov][(king.file() >= 4) as usize][BUCKETS[king as usize ^ (56 * pov as usize)]];

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

        unsafe { apply_changes(params, entry, adds, subs) };

        entry.pieces = board.pieces_bbs();
        entry.colors = board.colors_bbs();

        self.values[pov] = *entry.values;
        self.accurate[pov] = true;
    }

    pub fn update(&mut self, params: &Parameters, prev: &Self, board: &Board, king: Square, pov: Color) {
        let PstDelta { mv, piece, captured } = self.delta;

        let resulting_piece = mv.promotion_piece().unwrap_or_else(|| piece.piece_type());

        let add1 = pst_index(piece.piece_color(), resulting_piece, mv.to(), king, pov);
        let sub1 = pst_index(piece.piece_color(), piece.piece_type(), mv.from(), king, pov);

        if mv.is_castling() {
            let (rook_from, rook_to) = board.get_castling_rook(mv.to());

            let add2 = pst_index(piece.piece_color(), PieceType::Rook, rook_to, king, pov);
            let sub2 = pst_index(piece.piece_color(), PieceType::Rook, rook_from, king, pov);

            self.add2_sub2(params, prev, add1, add2, sub1, sub2, pov);
        } else if mv.is_capture() {
            let sub2 = if mv.is_en_passant() {
                pst_index(!piece.piece_color(), PieceType::Pawn, mv.to() ^ 8, king, pov)
            } else {
                pst_index(!piece.piece_color(), captured.piece_type(), mv.to(), king, pov)
            };

            self.add1_sub2(params, prev, add1, sub1, sub2, pov);
        } else {
            self.add1_sub1(params, prev, add1, sub1, pov);
        }

        self.accurate[pov] = true;
    }

    fn add1_sub1(&mut self, params: &Parameters, prev: &Self, add1: usize, sub1: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = params.ft_piece_weights[add1].as_ptr();
        let vsub1 = params.ft_piece_weights[sub1].as_ptr();

        for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
            unsafe {
                let mut v = *vprev.add(i).cast();
                v = simd::add_i16(v, *vadd1.add(i).cast());
                v = simd::sub_i16(v, *vsub1.add(i).cast());

                *vacc.add(i).cast() = v;
            }
        }
    }

    fn add1_sub2(&mut self, params: &Parameters, prev: &Self, add1: usize, sub1: usize, sub2: usize, pov: Color) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = params.ft_piece_weights[add1].as_ptr();
        let vsub1 = params.ft_piece_weights[sub1].as_ptr();
        let vsub2 = params.ft_piece_weights[sub2].as_ptr();

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

    fn add2_sub2(
        &mut self, params: &Parameters, prev: &Self, add1: usize, add2: usize, sub1: usize, sub2: usize, pov: Color,
    ) {
        let vacc = self.values[pov].as_mut_ptr();
        let vprev = prev.values[pov].as_ptr();

        let vadd1 = params.ft_piece_weights[add1].as_ptr();
        let vadd2 = params.ft_piece_weights[add2].as_ptr();
        let vsub1 = params.ft_piece_weights[sub1].as_ptr();
        let vsub2 = params.ft_piece_weights[sub2].as_ptr();

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

unsafe fn apply_changes(
    params: &Parameters, entry: &mut CacheEntry, adds: ArrayVec<usize, 32>, subs: ArrayVec<usize, 32>,
) {
    let mut registers: [_; REGISTERS] = std::mem::zeroed();

    for offset in (0..L1_SIZE).step_by(REGISTERS * simd::I16_LANES) {
        let output = entry.values.as_mut_ptr().add(offset);

        for (i, register) in registers.iter_mut().enumerate() {
            *register = *output.add(i * simd::I16_LANES).cast();
        }

        for &add in adds.iter() {
            let weights = params.ft_piece_weights[add].as_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = simd::add_i16(*register, *weights.add(i * simd::I16_LANES).cast());
            }
        }

        for &sub in subs.iter() {
            let weights = params.ft_piece_weights[sub].as_ptr().add(offset);

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
    let flip = (7 * ((king.file() >= 4) as u8)) ^ (56 * (pov as u8));

    BUCKETS[king ^ flip] * 768 + 384 * (color != pov) as usize + 64 * piece as usize + (square ^ flip) as usize
}

#[derive(Copy, Clone)]
pub struct ThreatDelta {
    piece: Piece,
    from: Square,
    attacked: Piece,
    to: Square,
    add: bool,
}

impl ThreatDelta {
    pub fn new(piece: Piece, from: Square, attacked: Piece, to: Square, add: bool) -> Self {
        Self { piece, from, attacked, to, add }
    }
}

#[derive(Clone)]
pub struct ThreatAccumulator {
    pub values: Aligned<[[i16; L1_SIZE]; 2]>,
    pub delta: ArrayVec<ThreatDelta, 80>,
    pub accurate: [bool; 2],
}

impl ThreatAccumulator {
    pub fn new() -> Self {
        Self {
            values: Aligned::new([[0; L1_SIZE]; 2]),
            delta: ArrayVec::new(),
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, params: &Parameters, board: &Board, pov: Color) {
        let king = board.king_square(pov);

        self.values[pov] = [0; L1_SIZE];

        for square in board.occupancies() {
            let piece = board.piece_on(square);
            let threats = attacks(piece, square, board.occupancies()) & board.occupancies();

            for target in threats {
                let attacked = board.piece_on(target);
                let mirrored = king.file() >= 4;

                let index = threat_index(piece, square, attacked, target, mirrored, pov);
                if index >= 0 {
                    unsafe { add1(params, &mut self.values[pov], index as usize) }
                }
            }
        }

        self.accurate[pov] = true;
    }

    pub unsafe fn update(&mut self, params: &Parameters, prev: &Self, king: Square, pov: Color) {
        let mut adds = ArrayVec::<usize, 256>::new();
        let mut subs = ArrayVec::<usize, 256>::new();

        for &ThreatDelta { piece, from, attacked, to, add } in self.delta.iter() {
            let mirrored = king.file() >= 4;

            let index = threat_index(piece, from, attacked, to, mirrored, pov);
            if add {
                adds.maybe_push(index >= 0, index as usize);
            } else {
                subs.maybe_push(index >= 0, index as usize);
            }
        }

        #[cfg(target_feature = "avx512f")]
        const REGISTERS: usize = L1_SIZE / simd::I16_LANES;
        #[cfg(not(target_feature = "avx512f"))]
        const REGISTERS: usize = 8;

        let mut registers: [_; REGISTERS] = std::mem::zeroed();

        for offset in (0..L1_SIZE).step_by(REGISTERS * simd::I16_LANES) {
            let input = prev.values[pov].as_ptr().add(offset);
            let output = self.values[pov].as_mut_ptr().add(offset);

            for (i, register) in registers.iter_mut().enumerate() {
                *register = *input.add(i * simd::I16_LANES).cast();
            }

            let mut add_idx = 0;
            let mut sub_idx = 0;

            while add_idx < adds.len() && sub_idx < subs.len() {
                let add = adds[add_idx];
                let sub = subs[sub_idx];

                let vadd = params.ft_threat_weights[add].as_ptr().add(offset);
                let vsub = params.ft_threat_weights[sub].as_ptr().add(offset);

                for (i, register) in registers.iter_mut().enumerate() {
                    let add_weights = simd::convert_i8_i16(*vadd.add(i * simd::I16_LANES).cast());
                    let sub_weights = simd::convert_i8_i16(*vsub.add(i * simd::I16_LANES).cast());
                    *register = simd::sub_i16(simd::add_i16(*register, add_weights), sub_weights);
                }

                add_idx += 1;
                sub_idx += 1;
            }

            while add_idx < adds.len() {
                let vadd = params.ft_threat_weights[adds[add_idx]].as_ptr().add(offset);

                for (i, register) in registers.iter_mut().enumerate() {
                    let add_weights = simd::convert_i8_i16(*vadd.add(i * simd::I16_LANES).cast());
                    *register = simd::add_i16(*register, add_weights);
                }

                add_idx += 1;
            }

            while sub_idx < subs.len() {
                let vsub = params.ft_threat_weights[subs[sub_idx]].as_ptr().add(offset);

                for (i, register) in registers.iter_mut().enumerate() {
                    let sub_weights = simd::convert_i8_i16(*vsub.add(i * simd::I16_LANES).cast());
                    *register = simd::sub_i16(*register, sub_weights);
                }

                sub_idx += 1;
            }

            for (i, register) in registers.iter().enumerate() {
                *output.add(i * simd::I16_LANES).cast() = *register;
            }
        }

        self.accurate[pov] = true;
    }
}

unsafe fn add1(params: &Parameters, output: &mut [i16], add1: usize) {
    let vacc = output.as_mut_ptr();
    let vadd1 = params.ft_threat_weights[add1].as_ptr();

    for i in (0..L1_SIZE).step_by(simd::I16_LANES) {
        let mut v = *vacc.add(i).cast();
        v = simd::add_i16(v, simd::convert_i8_i16(*vadd1.add(i).cast()));

        *vacc.add(i).cast() = v;
    }
}
