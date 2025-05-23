use std::{
    fmt::Display,
    ops::{Index, IndexMut, Not},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const NUM: usize = 2;
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
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> IndexMut<Color> for [T] {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::White => write!(f, "w"),
            Self::Black => write!(f, "b"),
        }
    }
}
