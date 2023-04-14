use crate::macros::impl_binary_op;

use super::Bitboard;

/// Represents a chess board square and bitboard element corresponding to a little-endian rank-file mapping.
///
/// See [LERFM](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Square(pub u8);

impl Square {
    pub const NUM: usize = 64;

    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);
    pub const F1: Self = Self(5);
    pub const G1: Self = Self(6);
    pub const H1: Self = Self(7);

    pub const A8: Self = Self(56);
    pub const B8: Self = Self(57);
    pub const C8: Self = Self(58);
    pub const E8: Self = Self(60);
    pub const D8: Self = Self(59);
    pub const F8: Self = Self(61);
    pub const G8: Self = Self(62);
    pub const H8: Self = Self(63);

    /// Contains little-endian rank-file square mappings.
    #[rustfmt::skip]
    const NOTATION: [&str; Square::NUM] = [
        "a1", "b1", "c1", "d1", "e1", "f1", "g1", "h1",
        "a2", "b2", "c2", "d2", "e2", "f2", "g2", "h2",
        "a3", "b3", "c3", "d3", "e3", "f3", "g3", "h3",
        "a4", "b4", "c4", "d4", "e4", "f4", "g4", "h4",
        "a5", "b5", "c5", "d5", "e5", "f5", "g5", "h5",
        "a6", "b6", "c6", "d6", "e6", "f6", "g6", "h6",
        "a7", "b7", "c7", "d7", "e7", "f7", "g7", "h7",
        "a8", "b8", "c8", "d8", "e8", "f8", "g8", "h8",
    ];

    /// Returns a `Square` from file and rank coordinates.
    ///
    /// # Panics
    ///
    /// Panics if rank or file is not in the range of `0..8`.
    #[inline(always)]
    pub fn from_axes(rank: u32, file: u32) -> Self {
        assert!((0..8).contains(&rank));
        assert!((0..8).contains(&file));

        Self(rank as u8 * 8 + file as u8)
    }

    /// Returns the bitboard containing the set bit at the current square value.
    #[inline(always)]
    pub fn to_bb(self) -> Bitboard {
        Bitboard(1 << self.0)
    }

    /// Returns a `Square` shifted by the specified offset.
    #[inline(always)]
    pub fn shift(self, offset: i8) -> Self {
        Self((self.0 as i8 + offset) as u8)
    }
}

impl_binary_op!(Square, Add, add);
impl_binary_op!(Square, u8, Div, div);

impl TryFrom<&str> for Square {
    type Error = ();

    /// Performs the conversion using the algebraic notation.
    ///
    /// The first character is defined to be only `a-h` / `A-H`.
    /// The second character is defined to be only `1-8`.

    /// # Examples
    ///
    /// ```
    /// use game::core::square::Square;
    ///
    /// assert_eq!(Square::try_from("a1"), Ok(Square(0)));
    /// assert_eq!(Square::try_from("C8"), Ok(Square(58)));
    /// assert_eq!(Square::try_from("k6"), Err(()));
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is invalid.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::NOTATION
            .iter()
            .position(|&v| v == value.to_lowercase())
            .map(|i| Square(i as u8))
            .ok_or(())
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", Self::NOTATION[self.0 as usize])
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

    #[test]
    fn try_from_str() {
        assert_eq!(Square::try_from("a1"), Ok(Square(0)));
        assert_eq!(Square::try_from("C8"), Ok(Square(58)));
        assert_eq!(Square::try_from("k6"), Err(()));
    }
}
