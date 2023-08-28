use game::Score;

use crate::QUIESCENCE_STAGES;

use super::AlphaBetaSearch;

impl<'a> AlphaBetaSearch<'a> {
    /// Performs a `negamax` search from the root node until the position becomes stable
    /// to evaluate it statically. This minimizes the horizon effect for volatile positions
    /// when threads and opportunities that go beyond the fixed depth of the search will
    /// remain undetected.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    pub fn quiescence_search(&mut self, mut alpha: Score, beta: Score) -> Score {
        if self.thread.get_terminator() {
            return Score::INVALID;
        }

        self.thread.nodes += 1;

        let evaluation = evaluation::evaluate_relative_score(self.board);

        if evaluation >= beta {
            return beta;
        }

        if evaluation > alpha {
            alpha = evaluation;
        }

        let mut moves = self.board.generate_moves();
        let mut ordering = self.build_ordering(QUIESCENCE_STAGES, &moves, None);

        while let Some(mv) = moves.next(&mut ordering) {
            if mv.is_capture() && self.board.make_move(mv).is_ok() {
                let score = -self.quiescence_search(-beta, -alpha);
                self.board.undo_move();

                if score >= beta {
                    return beta;
                }

                if score > alpha {
                    alpha = score;
                }
            }
        }

        alpha
    }
}
