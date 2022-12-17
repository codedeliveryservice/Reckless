use super::{macros::impl_ops, Square};

/// Represents a 64-bit unsigned integer with each bit indicating square occupancy
/// corresponding to a little-endian rank-file mapping.
///
/// See [Chess Programming Wiki article](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Bitboard(pub u64);

impl Bitboard {
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

impl_ops!(Bitboard);

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
}
