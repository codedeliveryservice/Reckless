use crate::{Zobrist, MAX_GAME_PLIES};

use super::state::State;

#[derive(Clone)]
pub(super) struct History {
    states: [State; MAX_GAME_PLIES],
    hashes: [Zobrist; MAX_GAME_PLIES],
    index: usize,
}

impl History {
    /// Pushes a state and hash to the history.
    pub fn push(&mut self, state: State, hash: Zobrist) {
        self.states[self.index] = state;
        self.hashes[self.index] = hash;
        self.index += 1;
    }

    /// Returns the last state and hash pushed to the history and removes them from the history.
    pub fn pop(&mut self) -> (State, Zobrist) {
        self.index -= 1;
        (self.states[self.index], self.hashes[self.index])
    }

    /// Returns `true` if the current position has been repeated at least once before.
    pub fn is_repetition(&self, hash: Zobrist) -> bool {
        (0..self.index).rev().any(|i| self.hashes[i] == hash)
    }
}

impl Default for History {
    fn default() -> Self {
        Self {
            states: [Default::default(); MAX_GAME_PLIES],
            hashes: [Default::default(); MAX_GAME_PLIES],
            index: 0,
        }
    }
}
