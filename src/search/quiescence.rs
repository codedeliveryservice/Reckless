use std::cmp::max;

use super::{alphabeta::AlphaBetaSearch, ordering::QUIESCENCE_STAGES};
use crate::{evaluation, types::Score};

impl<'a> AlphaBetaSearch<'a> {
    /// Performs a search until the position becomes stable enough for static evaluation.
    /// This minimizes the horizon effect for volatile positions, ensuring that threats
    /// and opportunities extending beyond the fixed search depth are not overlooked.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    pub fn quiescence_search(&mut self, mut alpha: Score, beta: Score) -> Score {
        self.thread.nodes += 1;

        if self.thread.get_terminator() {
            return Score::INVALID;
        }

        let static_score = evaluation::evaluate_relative_score(self.board);
        alpha = max(alpha, static_score);

        if alpha >= beta {
            return static_score;
        }

        let mut best_score = static_score;
        let mut moves = self.board.generate_moves();
        let mut ordering = self.build_ordering(QUIESCENCE_STAGES, &moves, None);

        while let Some(mv) = moves.next(&mut ordering) {
            if mv.is_capture() && self.board.make_move(mv).is_ok() {
                let score = -self.quiescence_search(-beta, -alpha);
                self.board.undo_move();

                best_score = max(best_score, score);
                alpha = max(alpha, score);

                if score >= beta {
                    break;
                }
            }
        }

        best_score
    }
}
