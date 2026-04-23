use super::{Aligned, L1_SIZE, simd};
use crate::{
    board::Board,
    lookup::attacks,
    nnue::Parameters,
    types::{ArrayVec, Color, Piece, Square},
};

mod threat_index;
pub use threat_index::*;

#[cfg(not(target_feature = "avx2"))]
mod scalar;
#[cfg(not(target_feature = "avx2"))]
pub use scalar::*;
#[cfg(target_feature = "avx2")]
mod vectorized;
#[cfg(target_feature = "avx2")]
pub use vectorized::*;

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct ThreatDelta(u32);

impl ThreatDelta {
    #[allow(dead_code)]
    pub const fn new(piece: Piece, from: Square, attacked: Piece, to: Square, add: bool) -> Self {
        Self(
            piece as u32
                | ((from as u32) << 8)
                | ((attacked as u32) << 16)
                | ((to as u32) << 24)
                | ((add as u32) << 31),
        )
    }

    pub const fn piece(self) -> Piece {
        unsafe { std::mem::transmute(self.0 as u8) }
    }

    pub const fn from(self) -> Square {
        unsafe { std::mem::transmute((self.0 >> 8) as u8) }
    }

    pub const fn attacked(self) -> Piece {
        unsafe { std::mem::transmute((self.0 >> 16) as u8) }
    }

    pub const fn to(self) -> Square {
        unsafe { std::mem::transmute(((self.0 >> 24) & 0x7F) as u8) }
    }

    pub const fn add(self) -> bool {
        self.0 >> 31 != 0
    }
}

#[derive(Clone)]
pub struct ThreatAccumulator {
    pub values: Aligned<[[i16; L1_SIZE]; 2]>,
    pub delta: ArrayVec<ThreatDelta, 80>,
    pub accurate: [bool; 2],
}

impl ThreatAccumulator {
    pub const fn new() -> Self {
        Self {
            values: Aligned::new([[0; L1_SIZE]; 2]),
            delta: ArrayVec::new(),
            accurate: [false; 2],
        }
    }

    pub fn refresh(&mut self, board: &Board, pov: Color, parameters: &Parameters) {
        let king = board.king_square(pov);

        let mut adds = ArrayVec::<usize, 8196>::new();

        for square in board.occupancies() {
            let piece = board.piece_on(square);
            let threats = attacks(piece, square, board.occupancies()) & board.occupancies();

            for target in threats {
                let attacked = board.piece_on(target);
                let mirrored = king.is_kingside();

                let index = threat_index(piece, square, attacked, target, mirrored, pov);
                adds.maybe_push(index >= 0, index as usize);
            }
        }

        #[cfg(target_feature = "avx512f")]
        const REGISTERS: usize = L1_SIZE / simd::I16_LANES;
        #[cfg(not(target_feature = "avx512f"))]
        const REGISTERS: usize = 8;

        unsafe {
            for offset in (0..L1_SIZE).step_by(REGISTERS * simd::I16_LANES) {
                let output = self.values[pov].as_mut_ptr().add(offset);

                let mut registers: [_; REGISTERS] = std::mem::zeroed();

                let mut add_idx = 0;

                while add_idx + 1 < adds.len() {
                    let add1 = adds[add_idx];
                    let add2 = adds[add_idx + 1];

                    let vadd1 = parameters.ft_threat_weights[add1].as_ptr().add(offset);
                    let vadd2 = parameters.ft_threat_weights[add2].as_ptr().add(offset);

                    for (i, register) in registers.iter_mut().enumerate() {
                        let add1_weights = simd::convert_i8_i16(*vadd1.add(i * simd::I16_LANES).cast());
                        let add2_weights = simd::convert_i8_i16(*vadd2.add(i * simd::I16_LANES).cast());
                        *register = simd::add_i16(*register, simd::add_i16(add1_weights, add2_weights));
                    }

                    add_idx += 2;
                }

                while add_idx < adds.len() {
                    let vadd = parameters.ft_threat_weights[adds[add_idx]].as_ptr().add(offset);

                    for (i, register) in registers.iter_mut().enumerate() {
                        let add_weights = simd::convert_i8_i16(*vadd.add(i * simd::I16_LANES).cast());
                        *register = simd::add_i16(*register, add_weights);
                    }

                    add_idx += 1;
                }

                for (i, register) in registers.iter().enumerate() {
                    *output.add(i * simd::I16_LANES).cast() = *register;
                }
            }
        }

        self.accurate[pov] = true;
    }

    pub unsafe fn update(&mut self, prev: &Self, king: Square, pov: Color, parameters: &Parameters) {
        let mut adds = ArrayVec::<usize, 256>::new();
        let mut subs = ArrayVec::<usize, 256>::new();

        for &td in self.delta.iter() {
            let (piece, from, attacked, to, add) = (td.piece(), td.from(), td.attacked(), td.to(), td.add());
            let mirrored = king.is_kingside();

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

                let vadd = parameters.ft_threat_weights[add].as_ptr().add(offset);
                let vsub = parameters.ft_threat_weights[sub].as_ptr().add(offset);

                for (i, register) in registers.iter_mut().enumerate() {
                    let add_weights = simd::convert_i8_i16(*vadd.add(i * simd::I16_LANES).cast());
                    let sub_weights = simd::convert_i8_i16(*vsub.add(i * simd::I16_LANES).cast());
                    *register = simd::add_i16(*register, simd::sub_i16(add_weights, sub_weights));
                }

                add_idx += 1;
                sub_idx += 1;
            }

            while add_idx < adds.len() {
                let vadd = parameters.ft_threat_weights[adds[add_idx]].as_ptr().add(offset);

                for (i, register) in registers.iter_mut().enumerate() {
                    let add_weights = simd::convert_i8_i16(*vadd.add(i * simd::I16_LANES).cast());
                    *register = simd::add_i16(*register, add_weights);
                }

                add_idx += 1;
            }

            while sub_idx < subs.len() {
                let vsub = parameters.ft_threat_weights[subs[sub_idx]].as_ptr().add(offset);

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
