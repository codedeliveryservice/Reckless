use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use super::{File, Rank, Square};

/// Represents a 64-bit unsigned integer with each bit indicating square occupancy.
///
/// See [Bitboards](https://www.chessprogramming.org/Bitboards) for more information.
#[derive(Copy, Clone, Eq, PartialEq, Default)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const ALL: Self = Self(0xFFFFFFFFFFFFFFFF);

    /// Creates a bitboard with all bits set in the specified rank.
    pub const fn rank(rank: Rank) -> Self {
        Self(0xFF << (rank as usize * 8))
    }

    pub const fn file(file: File) -> Self {
        Self(0x0101010101010101u64 << (file as usize))
    }

    /// Checks if the bitboard has zero bits set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square as u64) != 0
    }

    pub const fn multiple(self) -> bool {
        self.0 != 0 && self.0 & (self.0 - 1) != 0
    }

    /// Counts the number of set bits in the bitboard.
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns the least significant set bit in the bitboard.
    pub const fn lsb(self) -> Square {
        Square::new(self.0.trailing_zeros() as u8)
    }

    /// Shifts the bits of the bitboard by the specified offset.
    pub const fn shift(self, offset: i8) -> Self {
        if offset > 0 {
            Self(self.0 << offset)
        } else {
            Self(self.0 >> -offset)
        }
    }

    /// Sets the bit corresponding to the specified square.
    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square as u64;
    }

    /// Clears the bit corresponding to the specified square.
    pub fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square as u64);
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            None
        } else {
            let lsb = self.lsb();
            self.0 &= self.0 - 1;
            Some(lsb)
        }
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
