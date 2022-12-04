use super::square::Square;

/// Represents a 64-bit unsigned integer with each bit indicating square occupancy
/// corresponding to a little-endian rank-file mapping.
///
/// See [Chess Programming Wiki article](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// Determines whether the `Square` is set on the `Bitboard`.
    #[inline(always)]
    pub fn contains(self, square: Square) -> bool {
        (self.0 >> square.0) & 1 != 0
    }

    /// Sets the `Square` on the `Bitboard`.
    #[inline(always)]
    pub fn set(&mut self, square: Square) {
        self.0 |= 1 << square.0;
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
}
