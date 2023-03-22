use crate::MAX_GAME_PLIES;

use super::state::State;

#[derive(Clone)]
pub(super) struct History {
    list: [State; MAX_GAME_PLIES],
    index: usize,
}

impl History {
    /// Adds a state to the history list.
    pub fn push(&mut self, state: State) {
        self.list[self.index] = state;
        self.index += 1;
    }

    /// Removes the last state from the history list and returns it.
    pub fn pop(&mut self) -> State {
        self.index -= 1;
        self.list[self.index]
    }
}

impl Default for History {
    fn default() -> Self {
        Self {
            list: [Default::default(); MAX_GAME_PLIES],
            index: 0,
        }
    }
}
