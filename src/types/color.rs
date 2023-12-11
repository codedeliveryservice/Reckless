use std::ops::{Index, IndexMut, Not};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const NUM: usize = 2;

    /// Returns the difference between two adjacent ranks based on the current color.
    ///
    /// The result can only be `8` or `-8`.
    pub const fn offset(self) -> i8 {
        match self {
            Self::White => 8,
            Self::Black => -8,
        }
    }
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

impl<T> Index<Color> for [T] {
    type Output = T;

    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Color> for [T] {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
