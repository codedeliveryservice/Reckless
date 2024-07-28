use std::cmp::max;

use crate::{
    tables::{Bound, Entry},
    types::{Move, Piece, Score, MAX_PLY},
};

const PIECE_VALUES: [i32; 5] = [400, 750, 800, 1200, 1900];

impl super::SearchThread<'_> {
    /// Performs a search until the position becomes stable enough for static evaluation.
    /// This minimizes the horizon effect for volatile positions, ensuring that threats
    /// and opportunities extending beyond the fixed search depth are not overlooked.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    pub fn quiescence_search(&mut self, mut alpha: i32, beta: i32) -> i32 {
        self.nodes.inc();
        self.sel_depth = self.sel_depth.max(self.board.ply);

        // Prevent overflows
        if self.board.ply >= MAX_PLY - 1 {
            return self.board.evaluate();
        }

        let entry = self.tt.read(self.board.hash(), self.board.ply);
        if let Some(entry) = entry {
            if match entry.bound {
                Bound::Exact => true,
                Bound::Lower => entry.score >= beta,
                Bound::Upper => entry.score <= alpha,
            } {
                return entry.score;
            }
        }

        let mut eval = self.board.evaluate();

        if let Some(entry) = entry {
            adjust_eval(&mut eval, &entry);
        }

        alpha = max(alpha, eval);

        // The stand pat is the lower bound for the position, since doing nothing is *usually*
        // the least we can expect and it's already good enough to cause a beta cutoff
        if alpha >= beta {
            return eval;
        }

        let last_target = self.board.tail_move(1).target();

        let mut best_move = Move::NULL;
        let mut best_score = eval;

        let mut moves = self.board.generate_capture_moves();
        let mut ordering = self.build_ordering(&moves, None);

        while let Some(mv) = moves.next(&mut ordering) {
            if !mv.is_capture() {
                continue;
            }

            // Futility pruning
            #[cfg(not(feature = "datagen"))]
            if best_score > -Score::MATE_BOUND && mv.target() != last_target && eval + self.maximum_gain(mv) < alpha {
                break;
            }

            let key_after = self.board.key_after(mv);
            self.tt.prefetch(key_after);

            if self.board.make_move::<true>(mv).is_ok() {
                let score = -self.quiescence_search(-beta, -alpha);
                self.board.undo_move::<true>();

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
        self.tt.write(self.board.hash(), 0, best_score, bound, best_move, self.board.ply);
        best_score
    }

    /// Returns the material gain of a move.
    fn maximum_gain(&mut self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return PIECE_VALUES[Piece::Pawn];
        }

        let piece = self.board.piece_on(mv.target());

        if let Some(promotion) = mv.promotion_piece() {
            PIECE_VALUES[promotion] - PIECE_VALUES[Piece::Pawn] + PIECE_VALUES[piece]
        } else {
            PIECE_VALUES[piece]
        }
    }
}

fn adjust_eval(eval: &mut i32, entry: &Entry) {
    // If the TT entry has an exact bound or indicates that the current static evaluation
    // exceeds a bound (lower or upper), we can believe that the TT score is more accurate
    if match entry.bound {
        Bound::Exact => true,
        Bound::Lower => entry.score > *eval,
        Bound::Upper => entry.score < *eval,
    } && entry.score.abs() < Score::MATE_BOUND
    {
        *eval = entry.score;
    }
}
