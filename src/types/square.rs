use super::macros::impl_binary_op;

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

    /// Returns a `Square` from file and rank coordinates.
    pub fn from_rank_file(rank: u8, file: u8) -> Self {
        assert!(rank < 8);
        assert!(file < 8);
        Self(rank * 8 + file)
    }

    /// Returns a `Square` shifted by the specified offset.
    pub const fn shift(self, offset: i8) -> Self {
        Self((self.0 as i8 + offset) as u8)
    }
}

impl_binary_op!(Square, Add, add);
impl_binary_op!(Square, u8, Div, div);
impl_binary_op!(Square, u8, BitXor, bitxor);

impl TryFrom<&str> for Square {
    type Error = ();

    /// Performs the conversion using the algebraic notation.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let [file @ b'a'..=b'h', rank @ b'1'..=b'8'] = value.bytes().collect::<Vec<_>>().as_slice() {
            return Ok(Self::from_rank_file(rank - b'1', file - b'a'));
        }
        Err(())
    }
}

impl<T> std::ops::Index<Square> for [T] {
    type Output = T;

    fn index(&self, square: Square) -> &Self::Output {
        &self[square.0 as usize]
    }
}

impl<T> std::ops::IndexMut<Square> for [T] {
    fn index_mut(&mut self, square: Square) -> &mut Self::Output {
        &mut self[square.0 as usize]
    }
}

impl std::fmt::Display for Square {
    /// Formats the `Square` using the algebraic notation.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rank = self.0 / 8 + b'1';
        let file = self.0 % 8 + b'a';
        write!(f, "{}{}", file as char, rank as char)
    }
}

#[cfg(test)]
mod tests {
    use super::Square;

    #[test]
    fn try_from() {
        assert_eq!(Square::try_from("a1"), Ok(Square::A1));
        assert_eq!(Square::try_from("b1"), Ok(Square::B1));
        assert_eq!(Square::try_from("h8"), Ok(Square::H8));
        assert_eq!(Square::try_from("i9"), Err(()));
    }

    #[test]
    fn display() {
        assert_eq!(Square::A1.to_string(), "a1");
        assert_eq!(Square::B1.to_string(), "b1");
        assert_eq!(Square::H8.to_string(), "h8");
    }
}
