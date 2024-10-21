use std::cmp::max;

use crate::{
    tables::{Bound, Entry},
    types::{Move, MAX_PLY},
};

impl super::SearchThread<'_> {
    /// Performs a search until the position becomes stable enough for static evaluation.
    /// This minimizes the horizon effect for volatile positions, ensuring that threats
    /// and opportunities extending beyond the fixed search depth are not overlooked.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    pub fn quiescence_search(&mut self, mut alpha: i32, beta: i32) -> i32 {
        self.nodes.inc();
        self.sel_depth = self.sel_depth.max(self.ply);

        let entry = self.tt.read(self.board.hash(), self.ply);

        let eval = match entry {
            Some(entry) if should_cutoff(entry, alpha, beta) => return entry.score,
            Some(entry) => entry.score,
            None => self.board.evaluate() + self.corrhist.get(self.board),
        };

        if eval > alpha {
            alpha = eval;
        }

        if alpha >= beta || self.ply >= MAX_PLY - 1 {
            return eval;
        }

        let mut best_move = Move::NULL;
        let mut best_score = eval;

        let mut moves = self.board.generate_capture_moves();
        let mut ordering = self.build_ordering(&moves, None, 1);

        while let Some(mv) = moves.next(&mut ordering) {
            if !self.board.see(mv, 0) {
                continue;
            }

            let key_after = self.board.key_after(mv);
            self.tt.prefetch(key_after);

            if self.apply_move(mv) {
                let score = -self.quiescence_search(-beta, -alpha);
                self.revert_move();

                if score > best_score {
                    best_score = score;
                    best_move = mv;
                }

                alpha = max(alpha, score);

                if alpha >= beta {
                    break;
                }
            }
        }

        let bound = if best_score >= beta { Bound::Lower } else { Bound::Upper };
        self.tt.write(self.board.hash(), 0, best_score, bound, best_move, self.ply);
        best_score
    }
}

const fn should_cutoff(entry: Entry, alpha: i32, beta: i32) -> bool {
    match entry.bound {
        Bound::Exact => true,
        Bound::Lower => entry.score >= beta,
        Bound::Upper => entry.score <= alpha,
    }
}
