use super::{Piece, Square};

/// Represents a chess move containing the starting and target squares, as well as a kind for special moves.
#[derive(Clone, Copy)]
pub struct Move {
    start: Square,
    target: Square,
    kind: MoveKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Quiet,
    Capture,
    EnPassant,
    DoublePush,

    PromotionN,
    PromotionB,
    PromotionR,
    PromotionQ,

    PromotionCaptureN,
    PromotionCaptureB,
    PromotionCaptureR,
    PromotionCaptureQ,
}

impl Move {
    /// Creates an empty new `Move` which is invalid in chess.
    #[inline(always)]
    pub(crate) const fn null() -> Self {
        Self {
            start: Square(0),
            target: Square(0),
            kind: MoveKind::Quiet,
        }
    }

    /// Creates a new `Move`.
    #[inline(always)]
    pub(crate) fn new(start: Square, target: Square, kind: MoveKind) -> Self {
        Self {
            start,
            target,
            kind,
        }
    }

    /// Returns the start square of `self`.
    #[inline(always)]
    pub fn start(self) -> Square {
        self.start
    }

    /// Returns the target square of `self`.
    #[inline(always)]
    pub fn target(self) -> Square {
        self.target
    }

    /// Returns the kind of `self`.
    #[inline(always)]
    pub fn kind(self) -> MoveKind {
        self.kind
    }

    /// Returns `true` if the current move is a capture.
    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        matches!(
            self.kind,
            MoveKind::Capture
                | MoveKind::PromotionCaptureN
                | MoveKind::PromotionCaptureB
                | MoveKind::PromotionCaptureR
                | MoveKind::PromotionCaptureQ
        )
    }

    /// Returns `true` if the current move is a pawn promotion.
    #[inline(always)]
    pub const fn is_promotion(self) -> bool {
        matches!(
            self.kind,
            MoveKind::PromotionN
                | MoveKind::PromotionB
                | MoveKind::PromotionR
                | MoveKind::PromotionQ
                | MoveKind::PromotionCaptureN
                | MoveKind::PromotionCaptureB
                | MoveKind::PromotionCaptureR
                | MoveKind::PromotionCaptureQ
        )
    }

    /// Returns the piece to promote for the current move.
    ///
    /// # Panics
    ///
    /// Panics if the current move is not a pawn promotion.
    #[inline(always)]
    pub const fn get_promotion_piece(self) -> Piece {
        match self.kind {
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
        f.pad(&format!("{}{}", self.start(), self.target()))
    }
}
