use super::{Aligned, L1_SIZE, PARAMETERS, simd};
use crate::{
    nnue::INPUT_BUCKETS,
    types::{Bitboard, Color, PieceType},
};

pub mod psq;
pub mod threats;

pub use psq::PstAccumulator;
pub use threats::ThreatAccumulator;

#[derive(Clone, Default)]
pub struct AccumulatorCache {
    entries: Box<[[[CacheEntry; INPUT_BUCKETS]; 2]; 2]>,
}

#[derive(Clone)]
pub struct CacheEntry {
    values: Aligned<[i16; L1_SIZE]>,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
}

impl Default for CacheEntry {
    fn default() -> Self {
        Self {
            values: PARAMETERS.ft_biases.clone(),
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
        }
    }
}
