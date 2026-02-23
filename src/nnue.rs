mod accumulator;
mod threats;

pub use threats::initialize;

use crate::{
    board::{Board, BoardObserver},
    nnue::{
        accumulator::ThreatAccumulator,
        threats::{push_threats_on_change, push_threats_on_move, push_threats_on_mutate},
    },
    types::{Color, MAX_PLY, Move, Piece, PieceType, Square},
};

use accumulator::{AccumulatorCache, PstAccumulator};

mod forward {
    #[cfg(any(target_feature = "avx2", target_feature = "neon"))]
    mod vectorized;
    #[cfg(any(target_feature = "avx2", target_feature = "neon"))]
    pub use vectorized::*;

    #[cfg(not(any(target_feature = "avx2", target_feature = "neon")))]
    mod scalar;
    #[cfg(not(any(target_feature = "avx2", target_feature = "neon")))]
    pub use scalar::*;
}

mod simd {
    #[cfg(target_feature = "avx512f")]
    mod avx512;
    #[cfg(target_feature = "avx512f")]
    pub use avx512::*;

    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    mod avx2;
    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    pub use avx2::*;

    #[cfg(all(target_feature = "neon", not(any(target_feature = "avx2", target_feature = "avx512f"))))]
    mod neon;
    #[cfg(all(target_feature = "neon", not(any(target_feature = "avx2", target_feature = "avx512f"))))]
    pub use neon::*;

    #[cfg(not(any(target_feature = "avx512f", target_feature = "avx2", target_feature = "neon")))]
    mod scalar;
    #[cfg(not(any(target_feature = "avx512f", target_feature = "avx2", target_feature = "neon")))]
    pub use scalar::*;
}

const NETWORK_SCALE: i32 = 380;

const INPUT_BUCKETS: usize = 10;
const OUTPUT_BUCKETS: usize = 8;

const L1_SIZE: usize = 768;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;

const FT_QUANT: i32 = 255;
const L1_QUANT: i32 = 64;

#[cfg(target_feature = "avx512f")]
const FT_SHIFT: u32 = 9;
#[cfg(not(target_feature = "avx512f"))]
const FT_SHIFT: i32 = 9;

const DEQUANT_MULTIPLIER: f32 = (1 << FT_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

#[rustfmt::skip]
const INPUT_BUCKETS_LAYOUT: [usize; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,
    4, 5, 6, 7, 7, 6, 5, 4,
    8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
];

#[rustfmt::skip]
const OUTPUT_BUCKETS_LAYOUT: [usize; 33] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1,
    2, 2, 2, 2,
    3, 3, 3,
    4, 4, 4,
    5, 5, 5,
    6, 6, 6,
    7, 7, 7, 7,
];

#[repr(align(16))]
#[derive(Clone, Copy)]
struct SparseEntry {
    indexes: [u16; 8],
    count: usize,
}

#[derive(Clone)]
pub struct Network {
    index: usize,
    pst_stack: Box<[PstAccumulator]>,
    threat_stack: Box<[ThreatAccumulator]>,
    cache: AccumulatorCache,
    nnz_table: Box<[SparseEntry]>,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv.is_some());

        self.index += 1;

        self.pst_stack[self.index].accurate = [false; 2];
        self.pst_stack[self.index].delta.mv = mv;
        self.pst_stack[self.index].delta.piece = board.piece_on(mv.from());
        self.pst_stack[self.index].delta.captured = board.piece_on(mv.to());

        self.threat_stack[self.index].accurate = [false; 2];
        self.threat_stack[self.index].delta.clear();
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn full_refresh(&mut self, board: &Board) {
        self.pst_stack[self.index].refresh(board, Color::White, &mut self.cache);
        self.pst_stack[self.index].refresh(board, Color::Black, &mut self.cache);

        self.threat_stack[self.index].refresh(board, Color::White);
        self.threat_stack[self.index].refresh(board, Color::Black);
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        debug_assert!(self.pst_stack[0].accurate == [true; 2]);
        debug_assert!(self.threat_stack[0].accurate == [true; 2]);

        for pov in [Color::White, Color::Black] {
            if self.pst_stack[self.index].accurate[pov] && self.threat_stack[self.index].accurate[pov] {
                continue;
            }

            match self.can_update_pst(pov) {
                Some(index) => self.update_pst_accumulator(index, board, pov),
                None => self.pst_stack[self.index].refresh(board, pov, &mut self.cache),
            }

            match self.can_update_threats(pov) {
                Some(index) => self.update_threat_accumulator(index, board, pov),
                None => self.threat_stack[self.index].refresh(board, pov),
            }
        }

        self.output_transformer(board)
    }

    fn update_pst_accumulator(&mut self, accurate: usize, board: &Board, pov: Color) {
        let king = board.king_square(pov);

        for i in accurate..self.index {
            if let (prev, [current, ..]) = self.pst_stack.split_at_mut(i + 1) {
                current.update(&prev[i], board, king, pov);
            }
        }
    }

    fn update_threat_accumulator(&mut self, accurate: usize, board: &Board, pov: Color) {
        let king = board.king_square(pov);

        for i in accurate..self.index {
            if let (prev, [current, ..]) = self.threat_stack.split_at_mut(i + 1) {
                unsafe { current.update(&prev[i], king, pov) };
            }
        }
    }

    fn can_update_pst(&self, pov: Color) -> Option<usize> {
        for i in (0..=self.index).rev() {
            if self.pst_stack[i].accurate[pov] {
                return Some(i);
            }

            let delta = &self.pst_stack[i].delta;

            let from = delta.mv.from() ^ (56 * (delta.piece.piece_color() as u8));
            let to = delta.mv.to() ^ (56 * (delta.piece.piece_color() as u8));

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && ((from.file() >= 4) != (to.file() >= 4) || INPUT_BUCKETS_LAYOUT[from] != INPUT_BUCKETS_LAYOUT[to])
            {
                return None;
            }
        }

        None
    }

    fn can_update_threats(&self, pov: Color) -> Option<usize> {
        for i in (0..=self.index).rev() {
            if self.threat_stack[i].accurate[pov] {
                return Some(i);
            }

            let delta = &self.pst_stack[i].delta;

            let from = delta.mv.from();
            let to = delta.mv.to();

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && (from.file() >= 4) != (to.file() >= 4)
            {
                return None;
            }
        }

        None
    }

    fn output_transformer(&self, board: &Board) -> i32 {
        let bucket = OUTPUT_BUCKETS_LAYOUT[board.occupancies().popcount()];

        unsafe {
            let ft_out =
                forward::activate_ft(&self.pst_stack[self.index], &self.threat_stack[self.index], board.side_to_move());
            let (nnz_indexes, nnz_count) = forward::find_nnz(&ft_out, &self.nnz_table);

            let l1_out = forward::propagate_l1(ft_out, &nnz_indexes[..nnz_count], bucket);
            let l2_out = forward::propagate_l2(l1_out, bucket);
            let l3_out = forward::propagate_l3(l2_out, bucket);

            (l3_out * NETWORK_SCALE as f32) as i32
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        let mut nnz_table = vec![SparseEntry { indexes: [0; 8], count: 0 }; 256];

        for (byte, entry) in nnz_table.iter_mut().enumerate() {
            let mut count = 0;

            for bit in 0..8 {
                if (byte & (1 << bit)) != 0 {
                    entry.indexes[count] = bit as u16;
                    count += 1;
                }
            }

            entry.count = count;
        }

        Self {
            index: 0,
            pst_stack: vec![PstAccumulator::new(); MAX_PLY].into_boxed_slice(),
            threat_stack: vec![ThreatAccumulator::new(); MAX_PLY].into_boxed_slice(),
            cache: AccumulatorCache::default(),
            nnz_table: nnz_table.into_boxed_slice(),
        }
    }
}

impl BoardObserver for Network {
    fn on_piece_move(&mut self, board: &Board, piece: Piece, from: Square, to: Square) {
        push_threats_on_move(&mut self.threat_stack[self.index], board, piece, from, to);
    }

    fn on_piece_mutate(&mut self, board: &Board, old_piece: Piece, new_piece: Piece, square: Square) {
        push_threats_on_mutate(&mut self.threat_stack[self.index], board, old_piece, new_piece, square);
    }

    fn on_piece_change(&mut self, board: &Board, piece: Piece, square: Square, add: bool) {
        push_threats_on_change(&mut self.threat_stack[self.index], board, piece, square, add);
    }
}

#[repr(C)]
struct Parameters {
    ft_threat_weights: Aligned<[[i8; L1_SIZE]; 66864]>,
    ft_piece_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * 768]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[[i8; L2_SIZE * L1_SIZE]; OUTPUT_BUCKETS]>,
    l1_biases: Aligned<[[f32; L2_SIZE]; OUTPUT_BUCKETS]>,
    l2_weights: Aligned<[[[f32; L3_SIZE]; L2_SIZE]; OUTPUT_BUCKETS]>,
    l2_biases: Aligned<[[f32; L3_SIZE]; OUTPUT_BUCKETS]>,
    l3_weights: Aligned<[[f32; L3_SIZE]; OUTPUT_BUCKETS]>,
    l3_biases: Aligned<[f32; OUTPUT_BUCKETS]>,
}

static PARAMETERS: Parameters = unsafe { std::mem::transmute(*include_bytes!(env!("MODEL"))) };

#[repr(align(64))]
#[derive(Clone)]
struct Aligned<T> {
    data: T,
}

impl<T> Aligned<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
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
