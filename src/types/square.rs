/// Represents a chess board square and bitboard element corresponding to a little-endian rank-file mapping.
///
/// See [LERFM](https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping) for more information.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[rustfmt::skip]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
    #[default]
    None,
}

impl Square {
    pub const NUM: usize = 64;

    /// Returns the file of the `Square`.
    pub const fn file(self) -> usize {
        self as usize & 7
    }

    /// Returns a `Square` from file and rank coordinates.
    pub fn from_rank_file(rank: u8, file: u8) -> Self {
        ((rank << 3) | file).into()
    }

    /// Returns a `Square` shifted by the specified offset.
    pub fn shift(self, offset: i8) -> Self {
        ((self as i8 + offset) as u8).into()
    }
}

impl From<u8> for Square {
    fn from(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

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

impl std::ops::BitXor<u8> for Square {
    type Output = Self;

    fn bitxor(self, rhs: u8) -> Self::Output {
        (self as u8 ^ rhs).into()
    }
}

impl<T> std::ops::Index<Square> for [T] {
    type Output = T;

    fn index(&self, square: Square) -> &Self::Output {
        &self[square as usize]
    }
}

impl<T> std::ops::IndexMut<Square> for [T] {
    fn index_mut(&mut self, square: Square) -> &mut Self::Output {
        &mut self[square as usize]
    }
}

impl std::fmt::Display for Square {
    /// Formats the `Square` using the algebraic notation.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let rank = (*self as u8) / 8 + b'1';
        let file = (*self as u8) % 8 + b'a';
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
