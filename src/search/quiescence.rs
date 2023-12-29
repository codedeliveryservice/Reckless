use std::cmp::max;

use crate::{
    cache::Bound,
    evaluation::evaluate,
    types::{Move, Piece, MAX_PLY},
};

const MATERIAL: [i32; 5] = [175, 400, 450, 750, 1400];

impl super::Searcher<'_> {
    /// Performs a search until the position becomes stable enough for static evaluation.
    /// This minimizes the horizon effect for volatile positions, ensuring that threats
    /// and opportunities extending beyond the fixed search depth are not overlooked.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    pub fn quiescence_search(&mut self, mut alpha: i32, beta: i32) -> i32 {
        self.nodes += 1;
        self.sel_depth = self.sel_depth.max(self.board.ply);

        // Prevent overflows
        if self.board.ply >= MAX_PLY - 1 {
            return evaluate(&self.board);
        }

        let eval = evaluate(&self.board);
        alpha = max(alpha, eval);

        if alpha >= beta {
            return eval;
        }

        if let Some(entry) = self.cache.read(self.board.hash(), self.board.ply) {
            if match entry.bound {
                Bound::Exact => true,
                Bound::Lower => entry.score >= beta,
                Bound::Upper => entry.score <= alpha,
            } {
                return entry.score;
            }
        }

        let mut best_move = Move::NULL;
        let mut best_score = eval;

        let mut moves = self.board.generate_capture_moves();
        let mut ordering = self.build_ordering(&moves, None);

        while let Some(mv) = moves.next(&mut ordering) {
            if !mv.is_capture() {
                continue;
            }

            // Delta pruning
            if eval + self.gain(mv) < alpha {
                break;
            }

            if self.board.make_move(mv).is_ok() {
                let score = -self.quiescence_search(-beta, -alpha);
                self.board.undo_move();

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
        self.cache.write(self.board.hash(), 0, best_score, bound, best_move, self.board.ply);
        best_score
    }

    /// Returns the material gain of a move.
    fn gain(&mut self, mv: Move) -> i32 {
        let piece = self.board.get_piece(mv.target());

        if let Some(promo) = mv.get_promotion_piece() {
            MATERIAL[promo] - MATERIAL[Piece::Pawn] + MATERIAL[piece.unwrap()]
        } else {
            MATERIAL[piece.unwrap_or(Piece::Pawn)]
        }
    }
}
