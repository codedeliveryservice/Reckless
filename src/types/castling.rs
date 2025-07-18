use std::ops::{Index, IndexMut};

use super::Square;
use crate::{board::Board, types::Color};

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

    pub fn to_string(self, board: &Board) -> String {
        if self.raw == 0 {
            return "-".to_string();
        }

        let mut result = String::new();

        let kinds = [
            (CastlingKind::WhiteKinside, 'K', Color::White),
            (CastlingKind::WhiteQueenside, 'Q', Color::White),
            (CastlingKind::BlackKingside, 'k', Color::Black),
            (CastlingKind::BlackQueenside, 'q', Color::Black),
        ];

        for (kind, mut symbol, color) in kinds {
            if !self.is_allowed(kind) {
                continue;
            }

            if board.is_frc() {
                let (rook, _) = board.get_castling_rook(kind.landing_square());
                let base = match color {
                    Color::White => b'A',
                    Color::Black => b'a',
                };
                symbol = (rook.file() + base) as char;
            }
            result.push(symbol);
        }

        result
    }
}

impl<T> Index<Castling> for [T] {
    type Output = T;

    fn index(&self, index: Castling) -> &Self::Output {
        unsafe { self.get_unchecked(index.raw as usize) }
    }
}
