use std::ops::{Index, IndexMut};

use super::Color;

#[derive(Copy, Clone, PartialEq)]
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
    None,
}

impl Piece {
    pub const NUM: usize = 12;

    pub const fn new(color: Color, piece_type: PieceType) -> Self {
        unsafe { std::mem::transmute((piece_type as u8) << 1 | color as u8) }
    }

    pub const fn from_index(index: usize) -> Self {
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
        Ok(Piece::from_index(index))
    }
}

impl<T> Index<Piece> for [T] {
    type Output = T;

    fn index(&self, index: Piece) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Piece> for [T] {
    fn index_mut(&mut self, index: Piece) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Copy, Clone, PartialEq)]
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
}

impl<T> Index<PieceType> for [T] {
    type Output = T;

    fn index(&self, index: PieceType) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<PieceType> for [T] {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        &mut self[index as usize]
    }
}
