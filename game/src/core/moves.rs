use super::{Piece, Square};

/// Represents a chess move containing the starting and target squares, as well as flags for special moves.
/// The information encoded as a 16-bit integer, 6 bits for the start/target square and 4 bits for the flags.
///
/// See [Chess Programming Wiki article](https://www.chessprogramming.org/Encoding_Moves) for more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

/// Represents a typed enumeration of move kinds, which is the 4-bit part of the encoded bit move.
/// 
/// See [Chess Programming Wiki Article](https://www.chessprogramming.org/Encoding_Moves#From-To_Based) for more information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum MoveKind {
    Quiet             = 0b0000,
    DoublePush        = 0b0001,

    KingCastling      = 0b0010,
    QueenCastling     = 0b0011,

    Capture           = 0b0100,
    EnPassant         = 0b0101,

    PromotionN        = 0b1000,
    PromotionB        = 0b1001,
    PromotionR        = 0b1010,
    PromotionQ        = 0b1011,

    PromotionCaptureN = 0b1100,
    PromotionCaptureB = 0b1101,
    PromotionCaptureR = 0b1110,
    PromotionCaptureQ = 0b1111,
}

impl Move {
    const START_MASK: u16 = 0b0000_0000_0011_1111;
    const TARGET_MASK: u16 = 0b0000_1111_1100_0000;

    pub const EMPTY: Self = Self(0);

    pub const WHITE_SHORT_CASTLING: Self = Self(0b0010_0001_1000_0100);
    pub const WHITE_LONG_CASTLING: Self = Self(0b0011_0000_1000_0100);
    pub const BLACK_SHORT_CASTLING: Self = Self(0b0010_1111_1011_1100);
    pub const BLACK_LONG_CASTLING: Self = Self(0b0011_1110_1011_1100);

    /// Creates a new `Move`.
    #[inline(always)]
    pub(crate) fn new(start: Square, target: Square, kind: MoveKind) -> Self {
        Self(start.0 as u16 | (target.0 as u16) << 6 | (kind as u16) << 12)
    }

    /// Returns the start square of `self`.
    #[inline(always)]
    pub const fn start(self) -> Square {
        Square((self.0 & Self::START_MASK) as u8)
    }

    /// Returns the target square of `self`.
    #[inline(always)]
    pub const fn target(self) -> Square {
        Square(((self.0 & Self::TARGET_MASK) >> 6) as u8)
    }

    /// Returns the kind of `self`.
    #[inline(always)]
    pub const fn kind(self) -> MoveKind {
        unsafe { std::mem::transmute((self.0 >> 12) as u8) }
    }

    /// Returns `true` if the current move is a capture.
    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        (self.0 >> 14) & 1 != 0
    }

    /// Returns `true` if the current move is a pawn promotion.
    #[inline(always)]
    pub const fn is_promotion(self) -> bool {
        (self.0 >> 15) != 0
    }

    /// Returns the piece to promote for the current move.
    ///
    /// # Panics
    ///
    /// Panics if the current move is not a pawn promotion.
    #[inline(always)]
    pub const fn get_promotion_piece(self) -> Piece {
        match self.kind() {
            MoveKind::PromotionN | MoveKind::PromotionCaptureN => Piece::Knight,
            MoveKind::PromotionB | MoveKind::PromotionCaptureB => Piece::Bishop,
            MoveKind::PromotionR | MoveKind::PromotionCaptureR => Piece::Rook,
            MoveKind::PromotionQ | MoveKind::PromotionCaptureQ => Piece::Queen,
            _ => panic!("The move kind is not a promotion"),
        }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut output = format!("{}{}", self.start(), self.target());

        if self.is_promotion() {
            let ch = match self.get_promotion_piece() {
                Piece::Knight => 'n',
                Piece::Bishop => 'b',
                Piece::Rook => 'r',
                Piece::Queen => 'q',
                _ => panic!("The move was expected to be a promotion"),
            };
            output.push(ch);
        }

        f.pad(&output.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{Move, MoveKind, Square};

    macro_rules! assert_move {
        ($($name:ident: $kind:expr,)*) => {$(
            #[test]
            fn $name() {
                let mv = Move::new(Square(13), Square(47), $kind);

                assert_eq!(mv.start(), Square(13));
                assert_eq!(mv.target(), Square(47));
                assert_eq!(mv.kind(), $kind);
            }
        )*};
    }

    assert_move!(
        quiet: MoveKind::Quiet,
        capture: MoveKind::Capture,
        en_passant: MoveKind::EnPassant,
        double_push: MoveKind::DoublePush,
        king_castling: MoveKind::KingCastling,
        queen_castling: MoveKind::QueenCastling,
        knight_promotion: MoveKind::PromotionN,
        bishop_promotion: MoveKind::PromotionB,
        rook_promotion: MoveKind::PromotionR,
        queen_promotion: MoveKind::PromotionQ,
        knight_promotion_capture: MoveKind::PromotionCaptureN,
        bishop_promotion_capture: MoveKind::PromotionCaptureB,
        rook_promotion_capture: MoveKind::PromotionCaptureR,
        queen_promotion_capture: MoveKind::PromotionCaptureQ,
    );
}
