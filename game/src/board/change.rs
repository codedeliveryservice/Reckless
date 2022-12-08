use crate::core::{moves::Move, piece::Piece};

use super::state::State;

/// Contains the information required to unmake the move.
pub(super) struct Change {
    pub mv: Move,
    pub state: State,
    pub capture: Option<Piece>,
}

impl Change {
    /// Creates a new `Change`.
    #[inline(always)]
    pub(super) fn new(mv: Move, state: State, capture: Option<Piece>) -> Self {
        Self { mv, state, capture }
    }
}
