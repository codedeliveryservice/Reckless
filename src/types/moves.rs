use std::{mem, ops::Deref};

use super::{Piece, Square};

#[derive(Copy, Clone, PartialEq)]
pub struct FullMove {
    piece: Piece,
    inner: Move,
}

impl FullMove {
    pub const NULL: Self = Self { piece: Piece::None, inner: Move::NULL };

    pub const fn new(piece: Piece, mv: Move) -> Self {
        Self { piece, inner: mv }
    }

    pub const fn piece(self) -> Piece {
        self.piece
    }
}

impl Deref for FullMove {
    type Target = Move;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Represents a chess move containing the starting and target squares, as well as flags for special moves.
/// The information encoded as a 16-bit integer, 6 bits for the start/target square and 4 bits for the flags.
///
/// See [Encoding Moves](https://www.chessprogramming.org/Encoding_Moves) for more information.
#[derive(Copy, Clone, PartialEq)]
pub struct Move(u16);

/// Represents a typed enumeration of move kinds, which is the 4-bit part of the encoded bit move.
/// 
/// See [From-To Based](https://www.chessprogramming.org/Encoding_Moves#From-To_Based) for more information.
#[derive(Copy, Clone, PartialEq)]
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

    const START_MASK: u16 = 0b0000_0000_0011_1111;
    const TARGET_MASK: u16 = 0b0000_1111_1100_0000;

    /// Creates a new `Move`.
    pub const fn new(start: Square, target: Square, kind: MoveKind) -> Self {
        Self(start as u16 | (target as u16) << 6 | (kind as u16) << 12)
    }

    /// Returns the start square of `self`.
    pub const fn start(self) -> Square {
        unsafe { mem::transmute((self.0 & Self::START_MASK) as u8) }
    }

    /// Returns the target square of `self`.
    pub const fn target(self) -> Square {
        unsafe { mem::transmute(((self.0 & Self::TARGET_MASK) >> 6) as u8) }
    }

    /// Returns the kind of `self`.
    pub const fn kind(self) -> MoveKind {
        unsafe { mem::transmute((self.0 >> 12) as u8) }
    }

    /// Returns `true` if the current move is a capture.
    pub const fn is_capture(self) -> bool {
        (self.0 >> 14) & 1 != 0
    }

    /// Returns `true` if the current move is quiet.
    pub const fn is_quiet(self) -> bool {
        !self.is_capture()
    }

    /// Returns `true` if the current move is a pawn promotion.
    pub const fn is_promotion(self) -> bool {
        (self.0 >> 15) != 0
    }

    /// Returns `true` if the current move is an en passant capture.
    pub const fn is_en_passant(self) -> bool {
        matches!(self.kind(), MoveKind::EnPassant)
    }

    pub const fn is_castling(&self) -> bool {
        matches!(self.kind(), MoveKind::Castling)
    }

    pub const fn promotion_piece(self) -> Option<Piece> {
        match self.kind() {
            MoveKind::PromotionN | MoveKind::PromotionCaptureN => Some(Piece::Knight),
            MoveKind::PromotionB | MoveKind::PromotionCaptureB => Some(Piece::Bishop),
            MoveKind::PromotionR | MoveKind::PromotionCaptureR => Some(Piece::Rook),
            MoveKind::PromotionQ | MoveKind::PromotionCaptureQ => Some(Piece::Queen),
            _ => None,
        }
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut output = format!("{}{}", self.start(), self.target());

        match self.promotion_piece() {
            Some(Piece::Knight) => output.push('n'),
            Some(Piece::Bishop) => output.push('b'),
            Some(Piece::Rook) => output.push('r'),
            Some(Piece::Queen) => output.push('q'),
            _ => (),
        };

        f.pad(&output)
    }
}
