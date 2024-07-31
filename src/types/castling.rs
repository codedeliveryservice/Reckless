use std::ops::Index;

use super::{Bitboard, Move, MoveKind, Square};

pub trait CastlingKind {
    /// The mask of the castling kind.
    const MASK: u8;
    /// The mask of squares that must be empty for the castling to be legal.
    const PATH_MASK: Bitboard;
    /// The squares that must not be attacked for the castling to be legal.
    const CHECK_SQUARES: [Square; 2];
    /// The castling move associated with the castling kind.
    const CASTLING_MOVE: Move;
}

macro_rules! impl_castling_kind {
    ($($kind:ident => $raw:expr, $path_mask:expr, $start:expr, $adjacent: expr, $target:expr,)*)  => {
        $(
            pub struct $kind;

            impl CastlingKind for $kind {
                const MASK: u8 = $raw;
                const PATH_MASK: Bitboard = Bitboard($path_mask);
                const CHECK_SQUARES: [Square; 2] = [$start, $adjacent];
                const CASTLING_MOVE: Move = Move::new($start, $target, MoveKind::Castling);
            }
        )*
    };
}

impl_castling_kind! {
    WhiteKingSide   => 1, 0b0110_0000, Square::E1, Square::F1, Square::G1,
    WhiteQueenSide  => 2, 0b0000_1110, Square::E1, Square::D1, Square::C1,
    BlackKingSide   => 4, 0b0110_0000 << 56, Square::E8, Square::F8, Square::G8,
    BlackQueenSide  => 8, 0b0000_1110 << 56, Square::E8, Square::D8, Square::C8,
}

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Castling {
    raw: u8,
}

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

    /// Updates the castling rights based on the movement of a piece.
    pub fn update(&mut self, start: Square, target: Square) {
        self.raw &= Self::UPDATES[start] & Self::UPDATES[target];
    }

    /// Checks if a specific castling kind is allowed.
    pub const fn is_allowed<KIND: CastlingKind>(self) -> bool {
        (self.raw & KIND::MASK) != 0
    }
}

impl From<&str> for Castling {
    fn from(text: &str) -> Self {
        let mut castling = Self::default();
        for right in text.chars() {
            castling.raw |= match right {
                'K' => WhiteKingSide::MASK,
                'Q' => WhiteQueenSide::MASK,
                'k' => BlackKingSide::MASK,
                'q' => BlackQueenSide::MASK,
                _ => continue,
            };
        }
        castling
    }
}

impl<T> Index<Castling> for [T] {
    type Output = T;

    fn index(&self, index: Castling) -> &Self::Output {
        &self[index.raw as usize]
    }
}
