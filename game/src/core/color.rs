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
            Color::White => 8,
            Color::Black => -8,
        }
    }

    /// Return the opposite color of `self`.
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    /// Changes the color of `self` to the opposite.
    pub fn reverse(&mut self) {
        *self = self.opposite();
    }
}

impl<T> std::ops::Index<Color> for [T] {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}
