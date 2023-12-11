use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub const NUM: usize = 6;
}

impl From<usize> for Piece {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Pawn,
            1 => Self::Knight,
            2 => Self::Bishop,
            3 => Self::Rook,
            4 => Self::Queen,
            5 => Self::King,
            _ => panic!("Unexpected piece '{value}'"),
        }
    }
}

impl From<char> for Piece {
    fn from(value: char) -> Self {
        match value {
            'P' | 'p' => Self::Pawn,
            'N' | 'n' => Self::Knight,
            'B' | 'b' => Self::Bishop,
            'R' | 'r' => Self::Rook,
            'Q' | 'q' => Self::Queen,
            'K' | 'k' => Self::King,
            _ => panic!("Unexpected piece '{value}'"),
        }
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
