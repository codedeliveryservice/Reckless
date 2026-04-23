use super::{Aligned, L1_SIZE, Parameters, simd};
use crate::{
    nnue::INPUT_BUCKETS,
    types::{Bitboard, Color, PieceType},
};

pub mod psq;
pub mod threats;

pub use psq::PstAccumulator;
pub use threats::ThreatAccumulator;

#[derive(Clone)]
pub struct AccumulatorCache {
    entries: Box<[[[CacheEntry; INPUT_BUCKETS]; 2]; 2]>,
}

impl AccumulatorCache {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            entries: Box::new([[[CacheEntry::new(parameters); INPUT_BUCKETS]; 2]; 2]),
        }
    }
}

#[derive(Copy, Clone)]
pub struct CacheEntry {
    values: Aligned<[i16; L1_SIZE]>,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
}

impl CacheEntry {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            values: parameters.ft_biases,
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
        }
    }
}
