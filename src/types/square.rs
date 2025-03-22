use std::ops::{Add, BitXor, Div, Index, IndexMut};

use super::Bitboard;

/// Represents a square on a bitboard corresponding to the [Little-Endian Rank-File Mapping][LERFM].
///
/// [LERFM]: https://www.chessprogramming.org/Square_Mapping_Considerations#Little-Endian_Rank-File_Mapping
#[derive(Copy, Clone, PartialEq, Debug, Default)]
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

    /// Creates a new square from the given value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the value is in the range `0..64`.
    pub const fn new(value: u8) -> Self {
        unsafe { std::mem::transmute(value) }
    }

    /// Creates a square from the given rank and file.
    pub const fn from_rank_file(rank: u8, file: u8) -> Self {
        Self::new((rank << 3) | file)
    }

    pub const fn file(self) -> u8 {
        self as u8 & 7
    }

    pub const fn rank(self) -> u8 {
        self as u8 >> 3
    }

    /// Shifts the square by the given offset.
    pub const fn shift(self, offset: i8) -> Self {
        Self::new((self as i8 + offset) as u8)
    }

    pub const fn to_bb(self) -> Bitboard {
        Bitboard(1 << (self as u8))
    }
}

impl TryFrom<&str> for Square {
    type Error = ();

    /// Performs the conversion using the algebraic notation.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.as_bytes() {
            [file @ b'a'..=b'h', rank @ b'1'..=b'8'] => {
                let rank = rank - b'1';
                let file = file - b'a';
                Ok(Self::from_rank_file(rank, file))
            }
            _ => Err(()),
        }
    }
}

impl Add<Square> for Square {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self as u8 + rhs as u8)
    }
}

impl Div<i8> for Square {
    type Output = Self;

    fn div(self, rhs: i8) -> Self::Output {
        Self::new((self as i8 / rhs) as u8)
    }
}

impl BitXor<u8> for Square {
    type Output = Self;

    fn bitxor(self, rhs: u8) -> Self::Output {
        Self::new(self as u8 ^ rhs)
    }
}

impl<T> Index<Square> for [T] {
    type Output = T;

    fn index(&self, square: Square) -> &Self::Output {
        &self[square as usize]
    }
}

impl<T> IndexMut<Square> for [T] {
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
    use super::*;

    #[test]
    fn try_from_str() {
        assert_eq!(Square::try_from("a1"), Ok(Square::A1));
        assert_eq!(Square::try_from("e5"), Ok(Square::E5));
        assert_eq!(Square::try_from("h8"), Ok(Square::H8));

        assert!(Square::try_from("i1").is_err());
        assert!(Square::try_from("a9").is_err());
        assert!(Square::try_from("invalid").is_err());
    }

    #[test]
    fn display() {
        assert_eq!(Square::A1.to_string(), "a1");
        assert_eq!(Square::E5.to_string(), "e5");
        assert_eq!(Square::H8.to_string(), "h8");
    }
}
