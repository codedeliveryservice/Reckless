mod pst;
mod threat;

pub use pst::*;
pub use threat::*;

use crate::{
    nnue::{Aligned, INPUT_BUCKETS, L1_SIZE, PARAMETERS},
    types::{Bitboard, Color, PieceType},
};

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
