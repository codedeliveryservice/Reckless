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
    #[inline(always)]
    pub const fn offset(self) -> i8 {
        match self {
            Self::White => 8,
            Self::Black => -8,
        }
    }

    /// Return the opposite color of `self`.
    #[inline(always)]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    /// Changes the color of `self` to the opposite.
    #[inline(always)]
    pub fn reverse(&mut self) {
        *self = self.opposite();
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::White
    }
}

impl<T> std::ops::Index<Color> for [T] {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}
