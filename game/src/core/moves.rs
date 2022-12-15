use super::square::Square;

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
}

impl Move {
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
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(&format!("{}{}", self.start(), self.target()))
    }
}
