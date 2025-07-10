use std::{fmt::Display, ops::Index};

use super::{Bitboard, Move, MoveKind, Square};

pub trait CastlingKind {
    /// The raw bitmask representing this castling kind.
    const MASK: u8;
    /// Squares the king must traverse when castling.
    const PATH_MASK: Bitboard;
    /// Squares that must not be attacked for castling to be legal.
    const THREAT_MASK: Bitboard;
    /// The castling move associated with this castling kind.
    const CASTLING_MOVE: Move;
}

macro_rules! impl_castling_kind {
    ($($kind:ident => $raw:expr, $path_mask:expr, $threat_mask:expr, $from:expr, $to:expr;)*)  => {
        $(
            pub struct $kind;

            impl CastlingKind for $kind {
                const MASK: u8 = $raw;
                const PATH_MASK: Bitboard = Bitboard($path_mask);
                const THREAT_MASK: Bitboard = Bitboard($threat_mask);
                const CASTLING_MOVE: Move = Move::new($from, $to, MoveKind::Castling);
            }
        )*
    };
}

impl_castling_kind! {
    WhiteKingSide   => 1, 0b0110_0000, 0b0011_0000, Square::E1, Square::G1;
    WhiteQueenSide  => 2, 0b0000_1110, 0b0001_1000, Square::E1, Square::C1;
    BlackKingSide   => 4, 0b0110_0000 << 56, 0b0011_0000 << 56, Square::E8, Square::G8;
    BlackQueenSide  => 8, 0b0000_1110 << 56, 0b0001_1000 << 56, Square::E8, Square::C8;
}

#[derive(Copy, Clone, Default)]
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
    pub fn update(&mut self, from: Square, to: Square) {
        self.raw &= Self::UPDATES[from] & Self::UPDATES[to];
    }

    /// Checks if a specific castling kind is allowed.
    pub const fn is_allowed<KIND: CastlingKind>(self) -> bool {
        (self.raw & KIND::MASK) != 0
    }

    pub const fn raw(self) -> u8 {
        self.raw
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
        unsafe { self.get_unchecked(index.raw as usize) }
    }
}

impl Display for Castling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.raw() == 0 {
            return write!(f, "-");
        }

        if self.is_allowed::<WhiteKingSide>() {
            write!(f, "K")?;
        }
        if self.is_allowed::<WhiteQueenSide>() {
            write!(f, "Q")?;
        }
        if self.is_allowed::<BlackKingSide>() {
            write!(f, "k")?;
        }
        if self.is_allowed::<BlackQueenSide>() {
            write!(f, "q")?;
        }
        Ok(())
    }
}
