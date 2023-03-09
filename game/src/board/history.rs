use super::state::State;

#[derive(Clone)]
pub(super) struct History {
    list: [State; Self::MAX_GAME_PLIES],
    index: usize,
}

impl History {
    /// The maximum number of plies that can occur in a game.
    const MAX_GAME_PLIES: usize = 1024;

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
            list: [Default::default(); Self::MAX_GAME_PLIES],
            index: 0,
        }
    }
}
