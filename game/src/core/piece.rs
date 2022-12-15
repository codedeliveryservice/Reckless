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

impl From<u8> for Piece {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Pawn,
            1 => Self::Knight,
            2 => Self::Bishop,
            3 => Self::Rook,
            4 => Self::Queen,
            5 => Self::King,
            _ => panic!("Unexpected piece value '{}'", value),
        }
    }
}

impl<T> std::ops::Index<Piece> for [T] {
    type Output = T;

    #[inline(always)]
    fn index(&self, index: Piece) -> &Self::Output {
        &self[index as usize]
    }
}
