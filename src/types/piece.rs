use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use super::Color;

#[derive(Copy, Clone, Default, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum Piece {
    WhitePawn,
    BlackPawn,
    WhiteKnight,
    BlackKnight,
    WhiteBishop,
    BlackBishop,
    WhiteRook,
    BlackRook,
    WhiteQueen,
    BlackQueen,
    WhiteKing,
    BlackKing,
    #[default]
    None,
}

impl Piece {
    pub const NUM: usize = 12;

    pub const ALL: [Self; Self::NUM] = [
        Self::WhitePawn,
        Self::BlackPawn,
        Self::WhiteKnight,
        Self::BlackKnight,
        Self::WhiteBishop,
        Self::BlackBishop,
        Self::WhiteRook,
        Self::BlackRook,
        Self::WhiteQueen,
        Self::BlackQueen,
        Self::WhiteKing,
        Self::BlackKing,
    ];

    pub const fn value(self) -> i32 {
        self.piece_type().value()
    }

    pub const fn new(color: Color, piece_type: PieceType) -> Self {
        unsafe { std::mem::transmute(((piece_type as u8) << 1) | color as u8) }
    }

    pub const fn from_index(index: usize) -> Self {
        debug_assert!(index < Self::NUM);

        unsafe { std::mem::transmute(index as u8) }
    }

    pub const fn piece_color(self) -> Color {
        unsafe { std::mem::transmute((self as u8) & 1) }
    }

    pub const fn piece_type(self) -> PieceType {
        unsafe { std::mem::transmute((self as u8) >> 1) }
    }
}

impl TryFrom<char> for Piece {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let index = "PpNnBbRrQqKk".find(value).ok_or(())?;
        Ok(Self::from_index(index))
    }
}

impl TryInto<char> for Piece {
    type Error = ();

    fn try_into(self) -> Result<char, Self::Error> {
        let c = match self {
            Self::WhitePawn => 'P',
            Self::BlackPawn => 'p',
            Self::WhiteKnight => 'N',
            Self::BlackKnight => 'n',
            Self::WhiteBishop => 'B',
            Self::BlackBishop => 'b',
            Self::WhiteRook => 'R',
            Self::BlackRook => 'r',
            Self::WhiteQueen => 'Q',
            Self::BlackQueen => 'q',
            Self::WhiteKing => 'K',
            Self::BlackKing => 'k',
            Self::None => return Err(()),
        };
        Ok(c)
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let piece = match self {
            Self::WhitePawn => 'P',
            Self::BlackPawn => 'p',
            Self::WhiteKnight => 'N',
            Self::BlackKnight => 'n',
            Self::WhiteBishop => 'B',
            Self::BlackBishop => 'b',
            Self::WhiteRook => 'R',
            Self::BlackRook => 'r',
            Self::WhiteQueen => 'Q',
            Self::BlackQueen => 'q',
            Self::WhiteKing => 'K',
            Self::BlackKing => 'k',
            Self::None => panic!(),
        };
        write!(f, "{piece}")
    }
}

impl<T> Index<Piece> for [T] {
    type Output = T;

    fn index(&self, index: Piece) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> IndexMut<Piece> for [T] {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    None,
}

impl PieceType {
    pub const NUM: usize = 6;

    pub const fn new(value: usize) -> Self {
        unsafe { std::mem::transmute(value as u8) }
    }

    pub const fn value(self) -> i32 {
        match self {
            Self::Pawn => 109,
            Self::Knight => 403,
            Self::Bishop => 435,
            Self::Rook => 679,
            Self::Queen => 1242,
            Self::King => 0,
            Self::None => 0,
        }
    }
}

impl<T> Index<PieceType> for [T] {
    type Output = T;

    fn index(&self, index: PieceType) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> IndexMut<PieceType> for [T] {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}
