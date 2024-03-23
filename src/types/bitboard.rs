use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use super::{Rank, Square};

/// Represents a 64-bit unsigned integer with each bit indicating square occupancy.
///
/// See [Bitboards](https://www.chessprogramming.org/Bitboards) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// Creates a bitboard with all bits set in the specified rank.
    pub const fn rank(rank: Rank) -> Self {
        Self(0xFF << (rank as usize * 8))
    }

    /// Checks if the bitboard has zero bits set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Checks if the bitboard contains a set bit at the specified square position.
    pub const fn contains(self, square: Square) -> bool {
        (self.0 >> square as u64) & 1 != 0
    }

    /// Counts the number of set bits in the bitboard.
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Shifts the bits of the bitboard by the specified offset.
    pub fn shift(self, offset: i8) -> Self {
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

    /// Returns the least significant set bit in the bitboard.
    pub fn lsb(self) -> Square {
        Square::new(self.0.trailing_zeros() as u8)
    }

    /// Pops and returns the least significant set bit in the bitboard.
    ///
    /// `Square::None` is returned if the bitboard is empty.
    pub fn pop(&mut self) -> Square {
        let lsb = self.lsb();
        self.0 &= self.0 - 1;
        lsb
    }
}

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            None
        } else {
            Some(self.pop())
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

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}
