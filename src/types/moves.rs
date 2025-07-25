use std::mem;

use super::{PieceType, Square};
use crate::board::Board;

/// Represents a chess move containing the from and to squares, as well as flags for special moves.
/// The information encoded as a 16-bit integer, 6 bits for the from/to square and 4 bits for the flags.
///
/// See [Encoding Moves](https://www.chessprogramming.org/Encoding_Moves) for more information.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Move(u16);

/// Represents a typed enumeration of move kinds, which is the 4-bit part of the encoded bit move.
/// 
/// See [From-To Based](https://www.chessprogramming.org/Encoding_Moves#From-To_Based) for more information.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[rustfmt::skip]
pub enum MoveKind {
    Normal            = 0b0000,
    DoublePush        = 0b0001,
    Castling          = 0b0010,

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
    pub const NULL: Self = Self(0);

    pub const fn new(from: Square, to: Square, kind: MoveKind) -> Self {
        Self(from as u16 | ((to as u16) << 6) | ((kind as u16) << 12))
    }

    pub const fn from(self) -> Square {
        unsafe { mem::transmute((self.0 & 0b0000_0000_0011_1111) as u8) }
    }

    pub const fn to(self) -> Square {
        unsafe { mem::transmute(((self.0 & 0b0000_1111_1100_0000) >> 6) as u8) }
    }

    pub const fn encoded(self) -> usize {
        (self.0 & 0b0000_1111_1111_1111) as usize
    }

    pub const fn kind(self) -> MoveKind {
        unsafe { mem::transmute((self.0 >> 12) as u8) }
    }

    pub const fn is_some(self) -> bool {
        !self.is_null()
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub const fn is_quiet(self) -> bool {
        self.is_some() && !self.is_noisy()
    }

    pub const fn is_noisy(self) -> bool {
        matches!(
            self.kind(),
            MoveKind::Capture
                | MoveKind::EnPassant
                | MoveKind::PromotionQ
                | MoveKind::PromotionCaptureN
                | MoveKind::PromotionCaptureB
                | MoveKind::PromotionCaptureR
                | MoveKind::PromotionCaptureQ
        )
    }

    pub const fn is_capture(self) -> bool {
        (self.0 >> 14) & 1 != 0
    }

    pub const fn is_promotion(self) -> bool {
        (self.0 >> 15) != 0
    }

    pub const fn is_en_passant(self) -> bool {
        matches!(self.kind(), MoveKind::EnPassant)
    }

    pub const fn is_castling(self) -> bool {
        matches!(self.kind(), MoveKind::Castling)
    }

    pub const fn is_double_push(self) -> bool {
        matches!(self.kind(), MoveKind::DoublePush)
    }

    pub const fn promotion_piece(self) -> Option<PieceType> {
        match self.kind() {
            MoveKind::PromotionN | MoveKind::PromotionCaptureN => Some(PieceType::Knight),
            MoveKind::PromotionB | MoveKind::PromotionCaptureB => Some(PieceType::Bishop),
            MoveKind::PromotionR | MoveKind::PromotionCaptureR => Some(PieceType::Rook),
            MoveKind::PromotionQ | MoveKind::PromotionCaptureQ => Some(PieceType::Queen),
            _ => None,
        }
    }

    pub fn to_uci(self, board: &Board) -> String {
        // For FRC castling moves are encoded as king capturing rook
        if board.is_frc() && self.is_castling() {
            let king_from = self.from();
            let (rook_from, _) = board.get_castling_rook(self.to());
            return format!("{king_from}{rook_from}");
        }

        let mut output = format!("{}{}", self.from(), self.to());

        if self.is_promotion() {
            match self.promotion_piece() {
                Some(PieceType::Knight) => output.push('n'),
                Some(PieceType::Bishop) => output.push('b'),
                Some(PieceType::Rook) => output.push('r'),
                Some(PieceType::Queen) => output.push('q'),
                _ => (),
            }
        }

        output
    }
}
