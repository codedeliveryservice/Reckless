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
pub struct Bitboard(pub(crate) u64);

impl Bitboard {
    pub const RANK_1: Self = Self(0b1111_1111);
    pub const RANK_2: Self = Self(Self::RANK_1.0 << 8);
    pub const RANK_7: Self = Self(Self::RANK_1.0 << (8 * 6));

    pub const F1_G1: Self = Self(0b0110_0000);
    pub const F8_G8: Self = Self(Self::F1_G1.0 << (8 * 7));

    pub const B1_C1_D1: Self = Self(0b0000_1110);
    pub const B8_C8_D8: Self = Self(Self::B1_C1_D1.0 << (8 * 7));

    /// Returns `true` if `self` has zero bits set.
    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns `true` if `self` has one or more bits set.
    #[inline(always)]
    pub fn is_not_empty(self) -> bool {
        self.0 != 0
    }

    /// Returns `true` if `self` contains a set bit at the `Square` position.
    #[inline(always)]
    pub fn contains(self, square: Square) -> bool {
        (self.0 >> square.0) & 1 != 0
    }

    /// Returns the number of pieces on the `Bitboard`.
    #[inline(always)]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Sets the `Square` on the `Bitboard`.
    #[inline(always)]
    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square.0;
    }

    // Clears the `Square` on the `Bitboard`, if any.
    #[inline(always)]
    pub fn clear(&mut self, square: Square) {
        self.0 &= !(1 << square.0);
    }

    // Returns the least significant bit of the `Bitboard` and clears it, if any.
    #[inline(always)]
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

pub struct BitboardIter {
    bitboard: Bitboard,
}

impl Iterator for BitboardIter {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.bitboard.pop()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        BitboardIter { bitboard: self }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{bitboard::Bitboard, square::Square};

    #[test]
    fn contains() {
        let bb = Bitboard(0b110);

        assert_eq!(bb.contains(Square(1)), true);
        assert_eq!(bb.contains(Square(2)), true);
        assert_eq!(bb.contains(Square(3)), false);
    }

    #[test]
    fn set() {
        let mut bb = Bitboard(0b100);
        bb.set(Square(2));
        bb.set(Square(4));

        assert_eq!(Bitboard(0b10100), bb);
    }

    #[test]
    fn clear() {
        let mut bb = Bitboard(0b1010100);
        bb.clear(Square(0));
        bb.clear(Square(4));

        assert_eq!(Bitboard(0b1000100), bb);
    }

    #[test]
    fn pop() {
        let mut bb = Bitboard(0b10100);

        assert_eq!(bb.pop(), Some(Square(2)));
        assert_eq!(bb.pop(), Some(Square(4)));
        assert_eq!(bb.pop(), None);
    }

    #[test]
    fn iter() {
        let iter = Bitboard(0b110101).into_iter();
        let squares = [Square(0), Square(2), Square(4), Square(5)];

        assert!(iter.eq(squares.into_iter()));
    }
}
