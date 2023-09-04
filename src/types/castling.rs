use super::Square;

#[rustfmt::skip]
pub enum CastlingKind {
    WhiteShort = 0b0001,
    WhiteLong  = 0b0010,
    BlackShort = 0b0100,
    BlackLong  = 0b1000,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Castling(pub(crate) u8);

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
