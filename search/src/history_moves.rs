use game::Square;

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
pub struct HistoryMoves {
    table: [[u16; Square::NUM]; Square::NUM],
}

impl HistoryMoves {
    /// Increases the score of a move by the given depth.
    pub fn store(&mut self, start: Square, target: Square, depth: usize) {
        self.table[start.0 as usize][target.0 as usize] += depth as u16;
    }

    /// Returns the score of a move.
    pub fn get_score(&self, start: Square, target: Square) -> u16 {
        self.table[start.0 as usize][target.0 as usize]
    }
}

impl Default for HistoryMoves {
    fn default() -> Self {
        Self {
            table: [[0; Square::NUM]; Square::NUM],
        }
    }
}
