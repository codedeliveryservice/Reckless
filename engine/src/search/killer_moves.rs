use game::Move;

/// Provides an implementation of the killer heuristic used as a dynamic move
/// ordering technique for quiet moves that caused a beta cutoff.
///
/// See [Killer Heuristic](https://www.chessprogramming.org/Killer_Heuristic)
/// for more information.
pub struct KillerMoves<const SIZE: usize> {
    moves: [Move; SIZE],
}

impl<const SIZE: usize> KillerMoves<SIZE> {
    /// Creates a new `Killers<SIZE>`.
    pub fn new() -> Self {
        assert!(SIZE >= 1);

        Self {
            moves: [Default::default(); SIZE],
        }
    }

    /// Prepends the `Move` to the list of killer moves.
    pub fn add(&mut self, mv: Move) {
        for index in 1..self.moves.len() {
            self.moves[index] = self.moves[index - 1];
        }

        self.moves[0] = mv;
    }

    /// Returns `true` if `self` contains the specified killer `Move`.
    pub fn contains(&self, mv: Move) -> bool {
        self.moves.contains(&mv)
    }
}
