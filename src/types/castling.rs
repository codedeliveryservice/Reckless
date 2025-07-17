use std::{fmt::Display, ops::Index};

use super::{ Move, MoveKind, Square};

pub trait CastlingKind {
    /// The raw bitmask representing this castling kind.
    const MASK: u8;
    /// The castling move associated with this castling kind.
    const CASTLING_MOVE: Move;
}

macro_rules! impl_castling_kind {
    ($($kind:ident => $raw:expr, $from:expr, $to:expr;)*)  => {
        $(
            pub struct $kind;

            impl CastlingKind for $kind {
                const MASK: u8 = $raw;
                const CASTLING_MOVE: Move = Move::new($from, $to, MoveKind::Castling);
            }
        )*
    };
}

impl_castling_kind! {
    WhiteKingSide  => 1, Square::E1, Square::G1;
    WhiteQueenSide => 2, Square::E1, Square::C1;
    BlackKingSide  => 4, Square::E8, Square::G8;
    BlackQueenSide => 8, Square::E8, Square::C8;
}

#[derive(Copy, Clone, Default)]
pub struct Castling {
    pub raw: u8,
}

impl Castling {
    /// Checks if a specific castling kind is allowed.
    pub const fn is_allowed<KIND: CastlingKind>(self) -> bool {
        (self.raw & KIND::MASK) != 0
    }

    pub const fn raw(self) -> u8 {
        self.raw
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
