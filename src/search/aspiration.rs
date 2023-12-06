use super::Searcher;
use crate::types::Score;

const ASPIRATION_WINDOW: i32 = 30;

impl<'a> Searcher<'a> {
    pub fn aspiration_search(&mut self, mut score: i32, mut depth: i32) -> i32 {
        let original_depth = depth;

        let mut delta = (ASPIRATION_WINDOW - depth).max(10);
        let mut alpha = (score - delta).max(-Score::INFINITY);
        let mut beta = (score + delta).min(Score::INFINITY);

        loop {
            score = self.alpha_beta::<true, true>(alpha, beta, depth);

            if self.stopped {
                return 0;
            }

            if score <= alpha {
                alpha = (alpha - delta).max(-Score::INFINITY);
                beta = (alpha + beta) / 2;
                depth = original_depth;
            } else if score >= beta {
                beta = (beta + delta).min(Score::INFINITY);
                depth -= 1;
            } else {
                return score;
            }

            delta *= 2;
        }
    }
}
