use game::Move;

/// Provides an implementation of the killer heuristic used as a dynamic move
/// ordering technique for quiet moves that caused a beta cutoff.
///
/// See [Killer Heuristic](https://www.chessprogramming.org/Killer_Heuristic)
/// for more information.
pub struct KillerMoves<const SIZE: usize> {
    moves: [[Move; SIZE]; 64],
}

impl<const SIZE: usize> KillerMoves<SIZE> {
    /// Creates a new `Killers<SIZE>`.
    pub fn new() -> Self {
        assert!(SIZE >= 1);

        Self {
            moves: [[Default::default(); SIZE]; 64],
        }
    }

    /// Prepends the `Move` to the list of killer moves.
    pub fn add(&mut self, mv: Move, ply: usize) {
        for index in 1..self.moves[ply].len() {
            self.moves[ply][index] = self.moves[ply][index - 1];
        }

        self.moves[ply][0] = mv;
    }

    /// Returns `true` if `self` contains the specified killer `Move`.
    pub fn contains(&self, mv: Move, ply: usize) -> bool {
        self.moves[ply].contains(&mv)
    }
}
