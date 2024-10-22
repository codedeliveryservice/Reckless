use super::SearchThread;
use crate::{parameters::*, types::Score};

impl<'a> SearchThread<'a> {
    pub fn aspiration_search(&mut self, mut score: i32, depth: i32) -> i32 {
        let mut alpha = -Score::INFINITY;
        let mut beta = Score::INFINITY;

        let mut delta = aspiration_delta() + score.pow(2) / 10000;

        if depth >= aspiration_depth() {
            alpha = (score - delta).max(-Score::INFINITY);
            beta = (score + delta).min(Score::INFINITY);
        }

        let mut fail_high_count = 0;

        loop {
            let adjusted_depth = (depth - fail_high_count).max(1);
            score = self.alpha_beta::<true, true>(alpha, beta, adjusted_depth);

            if self.stopped {
                return 0;
            }

            if score <= alpha {
                alpha = (alpha - delta).max(-Score::INFINITY);
                beta = (alpha + beta) / 2;
                fail_high_count = 0;
            } else if score >= beta {
                beta = (beta + delta).min(Score::INFINITY);
                fail_high_count += 1;
            } else {
                return score;
            }

            delta += delta / 2;
        }
    }
}
