use super::selectivity::{futility_pruning, quiet_late_move_pruning};
use crate::{
    tables::{Bound, Entry},
    types::{Move, Score, MAX_PLY},
};

const DEEPER_SEARCH_MARGIN: i32 = 80;
const IIR_DEPTH: i32 = 4;

impl super::SearchThread<'_> {
    /// Performs an alpha-beta search in a fail-soft environment.
    pub fn alpha_beta<const PV: bool, const ROOT: bool>(&mut self, mut alpha: i32, mut beta: i32, mut depth: i32) -> i32 {
        self.pv_table.clear(self.board.ply);

        // The search has been stopped by the UCI or the time control
        if self.should_interrupt_search() {
            return Score::INVALID;
        }

        if !ROOT {
            // Draw detection (50-move rule, threefold repetition)
            if self.board.is_draw() {
                // Use a little randomness to avoid 3-fold repetition blindness
                return -1 + (self.nodes.local() as i32 & 0x2);
            }

            // Mate Distance Pruning
            alpha = alpha.max(-Score::MATE + self.board.ply as i32);
            beta = beta.min(Score::MATE - (self.board.ply as i32) - 1);

            if alpha >= beta {
                return alpha;
            }
        }
    
        // Prevent overflows
        if self.board.ply >= MAX_PLY - 1 {
            return self.board.evaluate();
        }

        let in_check = self.board.is_in_check();

        // Quiescence search at the leaf nodes, skip if in check to avoid horizon effect
        if depth <= 0 && !in_check {
            return self.quiescence_search(alpha, beta);
        }

        depth = depth.max(0);

        // Update UCI statistics after the quiescence search to avoid counting the same node twice
        self.nodes.inc();
        self.sel_depth = self.sel_depth.max(self.board.ply);

        // Transposition table lookup and potential cutoff
        let entry = self.tt.read(self.board.hash(), self.board.ply);
        if let Some(entry) = entry {
            if !PV && transposition_table_cutoff(entry, alpha, beta, depth) {
                return entry.score;
            }
        }

        // Internal Iterative Reductions. If no hash move is found in the TT, reduce the search depth
        // to counter a potentially poor move ordering that could slow down the search on higher depths
        if entry.is_none() && depth >= IIR_DEPTH {
            depth -= 1;
        }

        let eval = match entry {
            Some(entry) => entry.score,
            None if in_check => -Score::INFINITY,
            None => self.board.evaluate(),
        };

        let improving = !in_check && self.board.ply > 1 && eval > self.eval_stack[self.board.ply - 2];

        self.eval_stack[self.board.ply] = eval;

        // Reset the killer moves for child nodes
        self.killers[self.board.ply + 1] = [Move::NULL; 2];

        // Node pruning strategies prior to the move loop
        if !ROOT && !PV && !in_check {
            if let Some(score) = self.reverse_futility_pruning(depth, beta, eval, improving) {
                return score;
            }
            if let Some(score) = self.null_move_pruning::<PV>(depth, beta, eval) {
                return score;
            }
            if let Some(score) = self.razoring(depth, alpha, beta, eval) {
                return score;
            }
        }

        // Check extensions. Extend the search depth due to low branching
        // and the possibility of being in a forced sequence of moves
        depth += i32::from(in_check);

        let original_alpha = alpha;
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::NULL;

        let mut moves_played = 0;
        let mut quiets = Vec::with_capacity(32);
        let mut moves = self.board.generate_all_moves();
        let mut ordering = self.build_ordering(&moves, entry.map(|entry| entry.mv));

        while let Some(mv) = moves.next(&mut ordering) {
            #[cfg(not(feature = "datagen"))]
            if !ROOT && moves_played > 0 && best_score > -Score::MATE_BOUND {
                if !PV && mv.is_quiet() && futility_pruning(depth, alpha, eval) {
                    break;
                }
                if mv.is_quiet() && quiet_late_move_pruning(depth, quiets.len() as i32, improving) {
                    break;
                }
                if mv.is_capture() && depth < 6 && !self.see(mv, -100 * depth) {
                    continue;
                }
            }

            let key_after = self.board.key_after(mv);
            self.tt.prefetch(key_after);

            if self.board.make_move::<true>(mv).is_err() {
                continue;
            }

            let nodes_before = self.nodes.local();

            let score = if moves_played == 0 {
                -self.alpha_beta::<PV, false>(-beta, -alpha, depth - 1)
            } else {
                let reduction = self.calculate_reduction::<PV>(mv, depth, moves_played);
                self.principle_variation_search::<PV>(alpha, beta, depth, reduction, best_score)
            };

            self.board.undo_move::<true>();
            moves_played += 1;

            if ROOT {
                self.node_table.add(mv, self.nodes.local() - nodes_before);
            }

            // Early return to prevent processing potentially corrupted search results
            if self.stopped {
                return Score::INVALID;
            }

            if score > best_score {
                best_score = score;
                best_move = mv;

                if score > alpha {
                    alpha = score;
                    self.pv_table.update(self.board.ply, mv);
                }
            }

            if alpha >= beta {
                break;
            }

            if mv.is_quiet() {
                quiets.push(mv);
            }
        }

        // Checkmate and stalemate detection
        if moves_played == 0 {
            return if in_check { Score::mated_in(self.board.ply) } else { Score::DRAW };
        }

        let bound = get_bound(best_score, original_alpha, beta);
        if bound == Bound::Lower && best_move.is_quiet() {
            self.update_ordering_heuristics(depth, best_move, quiets);
        }

        self.tt.write(self.board.hash(), depth, best_score, bound, best_move, self.board.ply);
        best_score
    }

    /// Performs a Principal Variation Search (PVS), optimizing the search efforts by testing moves
    /// with a null window and re-searching when promising. It also applies late move reductions.
    fn principle_variation_search<const PV: bool>(&mut self, alpha: i32, beta: i32, depth: i32, reduction: i32, best_score: i32) -> i32 {
        let mut new_depth = depth - 1;

        // Null window search with possible late move reduction
        let mut score = -self.alpha_beta::<false, false>(-alpha - 1, -alpha, new_depth - reduction);

        // If the search fails and reduction applied, re-search with full depth
        if alpha < score && reduction > 0 {
            // Adjust the search depth based on results of the LMR search
            new_depth += i32::from(score > best_score + DEEPER_SEARCH_MARGIN);

            score = -self.alpha_beta::<false, false>(-alpha - 1, -alpha, new_depth);
        }

        // If the search fails again, proceed to a full window search with full depth
        if alpha < score && score < beta {
            score = -self.alpha_beta::<PV, false>(-beta, -alpha, new_depth);
        }

        score
    }

    /// Checks if the search should be interrupted.
    fn should_interrupt_search(&mut self) -> bool {
        // Finish at least one iteration to avoid returning a null move
        if self.finished_depth < 1 {
            return false;
        }

        if self.time_manager.is_time_up(self.nodes.local()) {
            self.stopped = true;
        }
        self.stopped
    }

    /// Updates the ordering heuristics to improve the move ordering in future searches.
    fn update_ordering_heuristics(&mut self, depth: i32, best_move: Move, quiets: Vec<Move>) {
        self.killers[self.board.ply][1] = self.killers[self.board.ply][0];
        self.killers[self.board.ply][0] = best_move;

        self.history.update_main(self.board.side_to_move, best_move, &quiets, depth);
        self.history.update_continuation(&self.board, best_move, &quiets, depth);
    }
}

/// Determines the score bound based on the best score and the original alpha-beta window.
fn get_bound(score: i32, alpha: i32, beta: i32) -> Bound {
    if score <= alpha {
        return Bound::Upper;
    }
    if score >= beta {
        return Bound::Lower;
    }
    Bound::Exact
}

/// Provides a score for a transposition table cutoff, if applicable.
fn transposition_table_cutoff(entry: Entry, alpha: i32, beta: i32, depth: i32) -> bool {
    if entry.depth < depth {
        return false;
    }
    // The score is outside the alpha-beta window, resulting in a cutoff
    match entry.bound {
        Bound::Exact => true,
        Bound::Lower => entry.score >= beta,
        Bound::Upper => entry.score <= alpha,
    }
}
