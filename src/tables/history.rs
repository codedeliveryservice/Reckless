use crate::types::{Move, Square};

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
pub struct HistoryMoves {
    table: [[i32; Square::NUM]; Square::NUM],
}

impl HistoryMoves {
    /// Returns the score of a move.
    pub fn get(&self, mv: Move) -> i32 {
        self.table[mv.start()][mv.target()]
    }

    /// Increases the score of a move based on the depth of the search.
    pub fn increase(&mut self, mv: Move, depth: i32) {
        self.table[mv.start()][mv.target()] += depth * depth;
    }

    /// Decreases the score of a move based on the depth of the search.
    pub fn decrease(&mut self, mv: Move, depth: i32) {
        self.table[mv.start()][mv.target()] -= depth * depth;
    }
}

impl Default for HistoryMoves {
    fn default() -> Self {
        Self {
            table: [[0; Square::NUM]; Square::NUM],
        }
    }
}
