use super::{
    macros::{impl_bit_assign_op, impl_bit_op},
    Color, Square,
};

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
    const UPDATES: [u8; 64] = [
        13, 15, 15, 15, 12, 15, 15, 14,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        15, 15, 15, 15, 15, 15, 15, 15,
        7, 15, 15, 15, 3, 15, 15, 11,
    ];

    pub const NONE: Self = Self(0);

    pub const WHITE_KING_SIDE: Self = Self(0b0001);
    pub const WHITE_QUEEN_SIDE: Self = Self(0b0010);
    pub const BLACK_KING_SIDE: Self = Self(0b0100);
    pub const BLACK_QUEEN_SIDE: Self = Self(0b1000);

    /// Updates castling rights when interacting with the given `square`.
    pub fn update_for_square(&mut self, square: Square) {
        self.0 &= Self::UPDATES[square.0 as usize];
    }

    /// Returns `true` if king side castling is available for the specified color.
    pub const fn is_king_side_available(self, color: Color) -> bool {
        match color {
            Color::White => (self.0 & Self::WHITE_KING_SIDE.0) != 0,
            Color::Black => (self.0 & Self::BLACK_KING_SIDE.0) != 0,
        }
    }

    /// Returns `true` if queen side castling is available for the specified color.
    pub const fn is_queen_side_available(self, color: Color) -> bool {
        match color {
            Color::White => (self.0 & Self::WHITE_QUEEN_SIDE.0) != 0,
            Color::Black => (self.0 & Self::BLACK_QUEEN_SIDE.0) != 0,
        }
    }
}

impl_bit_op!(Castling, BitAnd, bitand);
impl_bit_assign_op!(Castling, BitOrAssign, bitor_assign);
