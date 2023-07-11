use game::Score;

pub struct SearchParams {
    pub alpha: Score,
    pub beta: Score,
    pub depth: usize,
    pub null_move_allowed: bool,
}

impl SearchParams {
    pub fn new(alpha: Score, beta: Score, depth: usize) -> Self {
        Self {
            alpha,
            beta,
            depth,
            null_move_allowed: true,
        }
    }
}
