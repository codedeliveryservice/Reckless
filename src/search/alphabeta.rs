use super::{ordering::ALPHABETA_STAGES, Searcher};
use crate::cache::{Bound, CacheEntry};
use crate::evaluation::evaluate;
use crate::types::{Move, Score};

const RFP_MARGIN: i32 = 75;
const RFP_DEPTH: i32 = 8;
const NMP_DEPTH: i32 = 3;
const NMP_REDUCTION: i32 = 2;
const LMR_MOVE_COUNT: usize = 4;
const LMR_DEPTH: i32 = 3;

impl<'a> Searcher<'a> {
    /// Performs an alpha-beta search in a fail-soft environment.
    pub fn alpha_beta<const PV: bool, const ROOT: bool>(&mut self, mut alpha: i32, beta: i32, mut depth: i32) -> i32 {
        // The search has been stopped by the UCI or the time control
        if self.should_interrupt_search() {
            return Score::INVALID;
        }

        // Draw detection, excluding the root node to ensure a valid move is returned
        if !ROOT && (self.board.is_repetition() || self.board.is_fifty_move_draw()) {
            return Score::DRAW;
        }

        // Check extensions: extend the search depth due to low branching and the possibility of
        // being in a forced sequence of moves
        let in_check = self.board.is_in_check();
        depth += in_check as i32;

        // Quiescence search at the leaf nodes, skip if in check to avoid horizon effect
        if depth == 0 {
            return self.quiescence_search(alpha, beta);
        }

        // Update UCI statistics after the quiescence search to avoid counting the same node twice
        self.nodes += 1;
        self.sel_depth = self.sel_depth.max(self.board.ply);

        // Transposition table lookup and potential cutoff
        let entry = self.cache.read(self.board.hash(), self.board.ply);
        if let Some(entry) = entry {
            if let Some(score) = self.transposition_table_cutoff(entry, alpha, beta, depth) {
                return score;
            }
        }

        if !ROOT && !PV && !in_check {
            let static_score = evaluate(&self.board);

            // Reverse futility pruning: if the static evaluation of the current position is significantly
            // higher than beta at low depths, it's likely to be good enough to cause a beta cutoff
            if depth < RFP_DEPTH && static_score - RFP_MARGIN * depth > beta {
                return static_score;
            }

            // Null move pruning: if giving a free move to the opponent leads to a beta cutoff, it's highly
            // likely to result in a cutoff after a real move is made, so the current node can be pruned
            if depth >= NMP_DEPTH && static_score > beta && !self.board.is_last_move_null() {
                self.board.make_null_move();
                let score = -self.alpha_beta::<PV, false>(-beta, -beta + 1, depth - NMP_REDUCTION - 1);
                self.board.undo_move();

                if score >= beta {
                    return beta;
                }
            }
        }

        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::default();
        let mut bound = Bound::Upper;

        let mut moves_played = 0;
        let mut moves = self.board.generate_moves();
        let mut ordering = self.build_ordering(ALPHABETA_STAGES, &moves, entry.map(|entry| entry.mv));

        while let Some(mv) = moves.next(&mut ordering) {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            let score = match moves_played {
                // The first move is likely to be the best, so it's searched with a full window
                0 => -self.alpha_beta::<PV, false>(-beta, -alpha, depth - 1),
                // The remaining moves are searched with a null window and possible reductions
                _ => {
                    let reduction = self.calculate_lmr(mv, depth, moves_played, in_check);
                    self.principle_variation_search::<PV>(alpha, beta, depth, reduction)
                }
            };

            self.board.undo_move();
            moves_played += 1;

            // Early return to prevent processing potentially corrupted search results
            if self.stopped {
                return Score::INVALID;
            }

            if score > best_score {
                best_score = score;
                best_move = mv;
            }

            if score > alpha {
                alpha = score;
                bound = Bound::Exact;
            }

            if score >= beta {
                bound = Bound::Lower;

                if mv.is_quiet() {
                    self.killers.add(mv, self.board.ply);
                    self.history.update(mv, depth);
                }

                break;
            }
        }

        if moves_played == 0 {
            return self.final_score(in_check);
        }

        let entry = CacheEntry::new(self.board.hash(), depth, best_score, bound, best_move);
        self.cache.write(entry, self.board.ply);
        best_score
    }

    fn calculate_lmr(&self, mv: Move, depth: i32, moves_played: usize, in_check: bool) -> i32 {
        if !mv.is_capture() && !mv.is_promotion() && !in_check && moves_played >= LMR_MOVE_COUNT && depth >= LMR_DEPTH {
            2
        } else {
            0
        }
    }

    /// Performs a Principal Variation Search (PVS), optimizing the search efforts by testing moves
    /// with a null window and re-searching when promising. It also applies late move reductions.
    fn principle_variation_search<const PV: bool>(&mut self, alpha: i32, beta: i32, depth: i32, reduction: i32) -> i32 {
        // Null window search with possible late move reduction
        let mut score = -self.alpha_beta::<false, false>(-alpha - 1, -alpha, depth - reduction - 1);

        // If the search fails and reduction applied, re-search with full depth
        if alpha < score && reduction > 0 {
            score = -self.alpha_beta::<false, false>(-alpha - 1, -alpha, depth - 1);
        }

        // If the search fails again, proceed to a full window search with full depth
        if alpha < score && score < beta {
            score = -self.alpha_beta::<PV, false>(-beta, -alpha, depth - 1);
        }

        score
    }

    /// Checks if the search should be interrupted.
    fn should_interrupt_search(&mut self) -> bool {
        // Ensure a valid move is returned by completing at least one iteration of iterative deepening
        if self.nodes % 4096 == 0 && self.time_manager.is_time_over() {
            self.stopped = true;
        }
        self.stopped
    }

    /// Provides a score for a transposition table cutoff, if applicable.
    fn transposition_table_cutoff(&mut self, entry: CacheEntry, alpha: i32, beta: i32, depth: i32) -> Option<i32> {
        if entry.depth < depth as u8 {
            return None;
        }
        // The score is outside the alpha-beta window, resulting in a cutoff
        match entry.bound {
            Bound::Exact => Some(entry.score),
            Bound::Lower if entry.score >= beta => Some(entry.score),
            Bound::Upper if entry.score <= alpha => Some(entry.score),
            _ => None,
        }
    }

    /// Calculates the final score in case of a checkmate or stalemate.
    fn final_score(&mut self, in_check: bool) -> i32 {
        if in_check {
            -Score::CHECKMATE + self.board.ply as i32
        } else {
            Score::DRAW
        }
    }
}
