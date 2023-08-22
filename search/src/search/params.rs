use game::Score;

pub struct SearchParams {
    /// The lower bound of the score window for the alpha-beta pruning.
    pub alpha: Score,
    /// The upper bound of the score window for the alpha-beta pruning.
    pub beta: Score,
    /// The remaining search depth (height of the remaining search tree).
    pub depth: usize,
}

impl SearchParams {
    /// Creates a new `SearchParams`.
    pub fn new(alpha: Score, beta: Score, depth: usize) -> Self {
        Self { alpha, beta, depth }
    }
}
