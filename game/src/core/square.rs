use super::bitboard::Bitboard;

/// Represents a chess board square and bitboard element corresponding to a little-endian rank-file mapping.
///
/// See [Chess Programming Wiki article](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Square(pub u8);

impl Square {
    /// Returns a `Square` from file and rank coordinates.
    ///
    /// # Panics
    ///
    /// Panics if rank or file is not in the range of `0..8`.
    #[inline(always)]
    pub fn from_axes(rank: u32, file: u32) -> Square {
        assert!((0..8).contains(&rank));
        assert!((0..8).contains(&file));

        Square(rank as u8 * 8 + file as u8)
    }

    /// Returns the bitboard containing the set bit at the current square value.
    #[inline(always)]
    pub fn to_bb(self) -> Bitboard {
        Bitboard(1 << self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use crate::core::bitboard::Bitboard;

    use super::Square;

    #[test]
    fn from_axes() {
        assert_eq!(Square::from_axes(0, 3), Square(3));
        assert_eq!(Square::from_axes(2, 7), Square(23));

        assert!(catch_unwind(|| Square::from_axes(0, 8)).is_err());
        assert!(catch_unwind(|| Square::from_axes(8, 0)).is_err());
    }

    #[test]
    fn to_bb() {
        assert_eq!(Square(0).to_bb(), Bitboard(0b1));
        assert_eq!(Square(2).to_bb(), Bitboard(0b100));
        assert_eq!(Square(5).to_bb(), Bitboard(0b100000));
    }
}
