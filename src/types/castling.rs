use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use super::Square;

#[derive(Copy, Clone)]
pub enum CastlingKind {
    WhiteKinside = 0b0001,
    WhiteQueenside = 0b0010,
    BlackKingside = 0b0100,
    BlackQueenside = 0b1000,
}

impl CastlingKind {
    pub const fn landing_square(self) -> Square {
        match self {
            CastlingKind::WhiteKinside => Square::G1,
            CastlingKind::WhiteQueenside => Square::C1,
            CastlingKind::BlackKingside => Square::G8,
            CastlingKind::BlackQueenside => Square::C8,
        }
    }
}

impl<T> Index<CastlingKind> for [T] {
    type Output = T;

    fn index(&self, index: CastlingKind) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> IndexMut<CastlingKind> for [T] {
    fn index_mut(&mut self, index: CastlingKind) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(Copy, Clone, Default)]
pub struct Castling {
    pub raw: u8,
}

impl Castling {
    pub const fn raw(self) -> u8 {
        self.raw
    }

    pub const fn is_allowed(self, kind: CastlingKind) -> bool {
        (self.raw & kind as u8) != 0
    }
}

impl<T> Index<Castling> for [T] {
    type Output = T;

    fn index(&self, index: Castling) -> &Self::Output {
        unsafe { self.get_unchecked(index.raw as usize) }
    }
}

impl Display for Castling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.raw() == 0 {
            return write!(f, "-");
        }

        if self.is_allowed(CastlingKind::WhiteKinside) {
            write!(f, "K")?;
        }
        if self.is_allowed(CastlingKind::WhiteQueenside) {
            write!(f, "Q")?;
        }
        if self.is_allowed(CastlingKind::BlackKingside) {
            write!(f, "k")?;
        }
        if self.is_allowed(CastlingKind::BlackQueenside) {
            write!(f, "q")?;
        }
        Ok(())
    }
}
