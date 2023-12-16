use std::ops::Index;

use super::{Bitboard, Move, MoveKind, Square};

#[rustfmt::skip]
#[derive(Clone, Copy)]
pub enum CastlingKind {
    WhiteShort = 0b0001,
    WhiteLong  = 0b0010,
    BlackShort = 0b0100,
    BlackLong  = 0b1000,
}

impl CastlingKind {
    /// Returns the mask of squares that must be empty for the castling to be legal.
    pub fn path_mask(self) -> Bitboard {
        match self {
            Self::WhiteShort => Bitboard(0b0110_0000),
            Self::WhiteLong => Bitboard(0b0000_1110),
            Self::BlackShort => Bitboard(0b0110_0000 << 56),
            Self::BlackLong => Bitboard(0b0000_1110 << 56),
        }
    }

    /// Returns the squares that must not be attacked for the castling to be legal.
    /// 
    /// The mask includes the starting king square and excludes the ending king square.
    pub fn check_mask(self) -> [Square; 2] {
        match self {
            CastlingKind::WhiteShort => [Square::E1, Square::F1],
            CastlingKind::WhiteLong => [Square::E1, Square::D1],
            CastlingKind::BlackShort => [Square::E8, Square::F8],
            CastlingKind::BlackLong => [Square::E8, Square::D8],
        }
    }

    pub fn castling_move(self) -> Move {
        match self {
            CastlingKind::WhiteShort => Move::new(Square::E1, Square::G1, MoveKind::Castling),
            CastlingKind::WhiteLong => Move::new(Square::E1, Square::C1, MoveKind::Castling),
            CastlingKind::BlackShort => Move::new(Square::E8, Square::G8, MoveKind::Castling),
            CastlingKind::BlackLong => Move::new(Square::E8, Square::C8, MoveKind::Castling),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Castling(u8);

impl Castling {
    /// The update table contains masks for changing castling rights when moving
    /// from the square or to the square for any piece and is set as follows:
    /// ```md
    /// BIN      DEC       DESCRIPTION
    /// 0011       3       black both sides
    /// 0111       7       black queen side
    /// 1011      11       black king side
    /// 1100      12       white both sides
    /// 1101      13       white queen side
    /// 1110      14       white king side
    /// 1111      15       leave unchanged
    /// ```
    #[rustfmt::skip]
    const UPDATES: [u8; Square::NUM] = [
        13, 15, 15, 15, 12, 15, 15, 14,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
         7, 15, 15, 15,  3, 15, 15, 11,
    ];

    /// Updates castling rights when interacting with the `Square`.
    pub fn update_for_square(&mut self, square: Square) {
        self.0 &= Self::UPDATES[square];
    }

    /// Returns `true` if the `CastlingKind` is allowed.
    pub const fn is_allowed(self, kind: CastlingKind) -> bool {
        (self.0 & kind as u8) != 0
    }
}

impl From<&str> for Castling {
    fn from(text: &str) -> Self {
        let mut castling = Self::default();
        for right in text.chars() {
            castling.0 |= match right {
                'K' => CastlingKind::WhiteShort,
                'Q' => CastlingKind::WhiteLong,
                'k' => CastlingKind::BlackShort,
                'q' => CastlingKind::BlackLong,
                _ => continue,
            } as u8;
        }
        castling
    }
}

impl<T> Index<Castling> for [T] {
    type Output = T;

    fn index(&self, index: Castling) -> &Self::Output {
        &self[index.0 as usize]
    }
}
