use game::{Move, MAX_GAME_PLIES};

/// Provides an implementation of the killer heuristic used as a dynamic move
/// ordering technique for quiet moves that caused a beta cutoff.
///
/// See [Killer Heuristic](https://www.chessprogramming.org/Killer_Heuristic)
/// for more information.
pub struct KillerMoves {
    primary: [Move; MAX_GAME_PLIES],
    secondary: [Move; MAX_GAME_PLIES],
}

impl KillerMoves {
    /// Prepends the `Move` to the list of killer moves.
    #[inline(always)]
    pub fn add(&mut self, mv: Move, ply: usize) {
        self.secondary[ply] = self.primary[ply];
        self.primary[ply] = mv;
    }

    /// Returns `true` if `self` contains the specified killer `Move`.
    #[inline(always)]
    pub fn contains(&self, mv: Move, ply: usize) -> bool {
        self.primary[ply] == mv || self.secondary[ply] == mv
    }
}

impl Default for KillerMoves {
    fn default() -> Self {
        Self {
            primary: [Default::default(); MAX_GAME_PLIES],
            secondary: [Default::default(); MAX_GAME_PLIES],
        }
    }
}
