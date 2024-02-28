use super::selectivity::{futility_pruning, quiet_late_move_pruning};
use crate::{
    tables::{Bound, Entry},
    types::{Move, Score, MAX_PLY},
};

const IIR_DEPTH: i32 = 4;

impl super::Searcher<'_> {
    /// Performs an alpha-beta search in a fail-soft environment.
    pub fn alpha_beta<const PV: bool, const ROOT: bool>(&mut self, mut alpha: i32, beta: i32, mut depth: i32) -> i32 {
        self.pv_table.clear(self.board.ply);

        // The search has been stopped by the UCI or the time control
        if self.should_interrupt_search() {
            return Score::INVALID;
        }

        // Draw detection, excluding the root node to ensure a valid move is returned
        if !ROOT && self.board.is_draw() {
            return Score::DRAW;
        }

        // Prevent overflows
        if self.board.ply >= MAX_PLY - 1 {
            return self.board.evaluate();
        }

        // Check extensions: extend the search depth due to low branching and the possibility of
        // being in a forced sequence of moves
        let in_check = self.board.is_in_check();
        depth += i32::from(in_check);

        // Quiescence search at the leaf nodes, skip if in check to avoid horizon effect
        if depth <= 0 {
            return self.quiescence_search(alpha, beta);
        }

        // Update UCI statistics after the quiescence search to avoid counting the same node twice
        self.nodes += 1;
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

        let eval = entry.map_or_else(|| self.board.evaluate(), |entry| entry.score);
        let improving = !in_check && self.board.ply > 1 && eval > self.eval_stack[self.board.ply - 2];

        self.eval_stack[self.board.ply] = eval;

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

        let original_alpha = alpha;
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::NULL;

        let mut moves_played = 0;
        let mut quiets = Vec::with_capacity(32);
        let mut moves = self.board.generate_all_moves();
        let mut ordering = self.build_ordering(&moves, entry.map(|entry| entry.mv));

        while let Some(mv) = moves.next(&mut ordering) {
            if !ROOT && !PV && mv.is_quiet() && moves_played > 0 && best_score > -Score::MATE_BOUND {
                if futility_pruning(depth, alpha, eval) {
                    break;
                }
                if quiet_late_move_pruning(depth, quiets.len() as i32, improving) {
                    break;
                }
            }

            if self.board.make_move::<true>(mv).is_err() {
                continue;
            }

            let nodes_before = self.nodes;

            let score = if moves_played == 0 {
                // The first move is likely to be the best, so it's searched with a full window
                -self.alpha_beta::<PV, false>(-beta, -alpha, depth - 1)
            } else {
                // The remaining moves are searched with a null window and possible reductions
                let reduction = self.calculate_reduction::<PV>(mv, depth, moves_played);
                self.principle_variation_search::<PV>(alpha, beta, depth, reduction)
            };

            self.board.undo_move::<true>();
            moves_played += 1;

            if ROOT {
                self.node_table.add(mv, self.nodes - nodes_before);
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

        if moves_played == 0 {
            return self.final_score(in_check);
        }

        let bound = get_bound(best_score, original_alpha, beta);
        self.update_ordering_heuristics(depth, best_move, quiets, bound);
        self.tt.write(self.board.hash(), depth, best_score, bound, best_move, self.board.ply);
        best_score
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
        // Avoid pulling the timer too often to reduce the system call overhead
        const POLL_INTERVAL: u64 = 4096;

        // Finish at least one iteration to avoid returning a null move
        if self.finished_depth < 1 {
            return false;
        }

        if self.nodes >= self.time_manager.max_nodes() || (self.nodes % POLL_INTERVAL == 0 && self.time_manager.is_hard_bound_reached()) {
            self.stopped = true;
        }
        self.stopped
    }

    /// Updates the ordering heuristics to improve the move ordering in future searches.
    fn update_ordering_heuristics(&mut self, depth: i32, best_move: Move, quiets: Vec<Move>, bound: Bound) {
        if bound == Bound::Lower && best_move.is_quiet() {
            self.killers.add(best_move, self.board.ply);
            self.history.increase(best_move, depth);

            for mv in quiets {
                self.history.decrease(mv, depth);
            }
        }
    }

    /// Calculates the final score in case of a checkmate or stalemate.
    fn final_score(&mut self, in_check: bool) -> i32 {
        if in_check {
            -Score::MATE + self.board.ply as i32
        } else {
            Score::DRAW
        }
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
