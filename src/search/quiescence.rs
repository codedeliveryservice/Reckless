use std::cmp::max;

use crate::{
    parameters::OPT_PIECE_VALUES,
    tables::{Bound, Entry},
    types::{Move, Piece, Score, MAX_PLY},
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
            None => self.board.evaluate(),
        };

        if eval > alpha {
            alpha = eval;
        }

        if alpha >= beta || self.ply >= MAX_PLY - 1 {
            return eval;
        }

        let last_target = self.board.tail_move(1).target();

        let mut best_move = Move::NULL;
        let mut best_score = eval;

        let mut moves = self.board.generate_capture_moves();
        let mut ordering = self.build_ordering(&moves, None, 1);

        while let Some(mv) = moves.next(&mut ordering) {
            if !mv.is_capture() {
                continue;
            }

            // Pessimistic forward pruning
            #[cfg(not(feature = "datagen"))]
            if best_score > -Score::MATE_BOUND && mv.target() != last_target && eval + self.estimate_gain(mv) < alpha {
                break;
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

    /// Estimates the optimistic gain from a capture, i.e., ignoring possible piece loss afterward.
    fn estimate_gain(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return OPT_PIECE_VALUES[Piece::Pawn];
        }

        let piece = self.board.piece_on(mv.target());

        if let Some(promotion) = mv.promotion_piece() {
            OPT_PIECE_VALUES[promotion] - OPT_PIECE_VALUES[Piece::Pawn] + OPT_PIECE_VALUES[piece]
        } else {
            OPT_PIECE_VALUES[piece]
        }
    }
}

const fn should_cutoff(entry: Entry, alpha: i32, beta: i32) -> bool {
    match entry.bound {
        Bound::Exact => true,
        Bound::Lower => entry.score >= beta,
        Bound::Upper => entry.score <= alpha,
    }
}
