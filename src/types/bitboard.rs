use super::{
    macros::{impl_binary_op, impl_unary_op},
    Square,
};

/// Represents a 64-bit unsigned integer with each bit indicating square occupancy
/// corresponding to a little-endian rank-file mapping.
///
/// See [LERFM](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const RANK_1: Self = Self(0b1111_1111);
    pub const RANK_2: Self = Self(Self::RANK_1.0 << 8);
    pub const RANK_7: Self = Self(Self::RANK_1.0 << (8 * 6));

    /// Returns `true` if `self` has zero bits set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if `self` has one or more bits set.
    pub const fn is_not_empty(self) -> bool {
        self.0 != 0
    }

    /// Returns `true` if `self` contains a set bit at the `Square` position.
    pub const fn contains(self, square: Square) -> bool {
        (self.0 >> square.0) & 1 != 0
    }

    /// Returns the number of pieces on the `Bitboard`.
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Sets the `Square` on the `Bitboard`.
    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square.0;
    }

    // Clears the `Square` on the `Bitboard`, if any.
    pub fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square.0);
    }

    // Returns the least significant bit of the `Bitboard` and clears it, if any.
    pub fn pop(&mut self) -> Option<Square> {
        if self.is_empty() {
            return None;
        }

        let square = Square(self.0.trailing_zeros() as u8);
        self.clear(square);

        Some(square)
    }
}

impl_binary_op!(Bitboard, BitAnd, bitand);
impl_binary_op!(Bitboard, BitOr, bitor);
impl_unary_op!(Bitboard, Not, not);

impl Iterator for Bitboard {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}
