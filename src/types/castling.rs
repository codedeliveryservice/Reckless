use std::ops::Index;

use super::Square;

#[derive(Copy, Clone)]
#[rustfmt::skip]
pub enum CastlingKind {
    WhiteKingside  = 0b0001,
    WhiteQueenside = 0b0010,
    BlackKingside  = 0b0100,
    BlackQueenside = 0b1000,
}

impl CastlingKind {
    pub fn landing_square(&self) -> Square {
        match self {
            CastlingKind::WhiteKingside => Square::G1,
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

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct CastlingRights {
    pub raw: u8,
}

impl CastlingRights {
    pub fn set(&mut self, kind: CastlingKind) {
        match kind {
            CastlingKind::WhiteKingside => self.raw |= 0b0001,
            CastlingKind::WhiteQueenside => self.raw |= 0b0010,
            CastlingKind::BlackKingside => self.raw |= 0b0100,
            CastlingKind::BlackQueenside => self.raw |= 0b1000,
        }
    }

    pub fn is_allowed(&self, kind: CastlingKind) -> bool {
        self.raw & kind as u8 != 0
    }
}

impl<T> Index<CastlingRights> for [T] {
    type Output = T;

    fn index(&self, index: CastlingRights) -> &Self::Output {
        unsafe { self.get_unchecked(index.raw as usize) }
    }
}
