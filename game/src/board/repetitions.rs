use crate::{Zobrist, MAX_GAME_PLIES};

#[derive(Clone)]
pub(super) struct Repetitions {
    table: [Zobrist; MAX_GAME_PLIES],
    index: usize,
}

impl Repetitions {
    /// Adds a hash to the repetition table.
    pub fn push(&mut self, hash: Zobrist) {
        self.table[self.index] = hash;
        self.index += 1;
    }

    /// Removes the last hash from the repetition table and returns it.
    pub fn pop(&mut self) -> Zobrist {
        self.index -= 1;
        self.table[self.index]
    }

    /// Returns `true` if the given `Zobrist` hash was found in the repetition table.
    pub fn is_repetition(&self, hash: Zobrist) -> bool {
        (0..self.index).rev().any(|index| self.table[index] == hash)
    }
}

impl Default for Repetitions {
    fn default() -> Self {
        Self {
            table: [Default::default(); MAX_GAME_PLIES],
            index: 0,
        }
    }
}
