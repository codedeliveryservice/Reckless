use crate::types::{Move, MAX_SEARCH_PLY};

/// Provides an implementation of the killer heuristic used as a dynamic move
/// ordering technique for quiet moves that caused a beta cutoff.
///
/// See [Killer Heuristic](https://www.chessprogramming.org/Killer_Heuristic)
/// for more information.
pub struct KillerMoves {
    table: [[Move; 2]; MAX_SEARCH_PLY],
}

impl KillerMoves {
    /// Prepends the `Move` to the list of killer moves.
    pub fn add(&mut self, mv: Move, ply: usize) {
        self.table[ply][1] = self.table[ply][0];
        self.table[ply][0] = mv;
    }

    /// Returns `true` if `self` contains the specified killer `Move`.
    pub fn contains(&self, mv: Move, ply: usize) -> bool {
        self.table[ply][0] == mv || self.table[ply][1] == mv
    }
}

impl Default for KillerMoves {
    fn default() -> Self {
        Self {
            table: [[Move::NULL; 2]; MAX_SEARCH_PLY],
        }
    }
}
