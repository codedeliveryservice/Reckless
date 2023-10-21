use super::{ordering::ALPHABETA_STAGES, Searcher};
use crate::cache::{Bound, CacheEntry};
use crate::evaluation::evaluate;
use crate::types::{Move, Score};

impl Searcher {
    /// Performs an alpha-beta search in a fail-soft environment.
    pub fn alpha_beta(&mut self, alpha: i32, beta: i32, mut depth: usize) -> i32 {
        // The search has been stopped by the UCI or the time control
        if self.should_interrupt_search() {
            return Score::INVALID;
        }

        // Draw detection, excluding the root node to ensure a valid move is returned
        if !self.root() && (self.board.is_repetition() || self.board.is_fifty_move_draw()) {
            return Score::DRAW;
        }

        // Check extensions: extend the search depth due to low branching and the possibility of
        // being in a forced sequence of moves
        let in_check = self.board.is_in_check();
        depth += in_check as usize;

        // Quiescence search at the leaf nodes, skip if in check to avoid horizon effect
        if depth == 0 {
            return self.quiescence_search(alpha, beta);
        }

        // Update UCI statistics after the quiescence search to avoid counting the same node twice
        self.nodes += 1;
        self.sel_depth = self.sel_depth.max(self.board.ply);

        // Transposition table lookup and potential cutoff
        let entry = self.read_cache_entry();
        if let Some(entry) = entry {
            if let Some(score) = self.transposition_table_cutoff(entry, alpha, beta, depth) {
                return score;
            }
        }

        let pv_node = beta - alpha > 1;
        let static_score = evaluate(&self.board);

        if !self.root() && !pv_node && !in_check {
            const RFP_MARGIN: i32 = 75;

            // Reverse futility pruning: if the static evaluation of the current position is significantly
            // higher than beta at low depths, it's likely to be good enough to cause a beta cutoff
            if depth < 8 && static_score - RFP_MARGIN * depth as i32 > beta {
                return static_score;
            }

            // Null move pruning: if giving a free move to the opponent leads to a beta cutoff, it's highly
            // likely to result in a cutoff after a real move is made, so the current node can be pruned
            if depth >= 3 && static_score > beta && !self.board.is_last_move_null() {
                self.board.make_null_move();
                let score = -self.alpha_beta(-beta, -beta + 1, depth - 3);
                self.board.undo_move();

                if score >= beta {
                    return beta;
                }
            }
        }

        self.search_moves(alpha, beta, depth, in_check, entry.map(|entry| entry.mv))
    }

    fn search_moves(&mut self, mut alpha: i32, beta: i32, depth: usize, in_check: bool, cache_move: Option<Move>) -> i32 {
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::default();
        let mut bound = Bound::Upper;

        let mut moves_played = 0;
        let mut moves = self.board.generate_moves();
        let mut ordering = self.build_ordering(ALPHABETA_STAGES, &moves, cache_move);

        while let Some(mv) = moves.next(&mut ordering) {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            let is_quiet = !mv.is_capture() && !mv.is_promotion();
            let apply_lmr = is_quiet && !in_check && moves_played >= 4 && depth >= 3;
            let reduction = if apply_lmr { 3 } else { 1 };

            let score = self.principle_variation_search(alpha, beta, depth, reduction, moves_played);

            self.board.undo_move();
            moves_played += 1;

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

        self.write_cache_entry(depth, best_score, bound, best_move);
        best_score
    }

    /// Performs a Principal Variation Search (PVS), optimizing the search efforts by testing moves
    /// with a null window and re-searching when promising. It also applies late move reductions.
    fn principle_variation_search(&mut self, alpha: i32, beta: i32, depth: usize, reduction: usize, moves_played: usize) -> i32 {
        // The first move is likely to be the best, so it's searched with a full window
        if moves_played == 0 {
            return -self.alpha_beta(-beta, -alpha, depth - 1);
        }

        // Null window search with possible late move reduction
        let mut score = -self.alpha_beta(-alpha - 1, -alpha, depth - reduction);

        // If the search fails and reduction applied, re-search with full depth
        if alpha < score && reduction > 1 {
            score = -self.alpha_beta(-alpha - 1, -alpha, depth - 1);
        }

        // If the search fails again, proceed to a full window search with full depth
        if alpha < score && score < beta {
            score = -self.alpha_beta(-beta, -alpha, depth - 1);
        }

        score
    }

    /// Checks if the search should be interrupted.
    fn should_interrupt_search(&mut self) -> bool {
        // Ensure a valid move is returned by completing at least one iteration of iterative deepening
        if self.nodes % 4096 == 0 && self.time_manager.is_time_over() {
            self.store_terminator(true);
            return true;
        }
        false
    }

    /// Provides a score for a transposition table cutoff, if applicable.
    fn transposition_table_cutoff(&mut self, entry: CacheEntry, alpha: i32, beta: i32, depth: usize) -> Option<i32> {
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

    /// Reads a cache entry from the transposition table.
    fn read_cache_entry(&self) -> Option<CacheEntry> {
        self.cache.lock().unwrap().read(self.board.hash(), self.board.ply)
    }

    /// Writes a new cache entry to the transposition table.
    fn write_cache_entry(&mut self, depth: usize, score: i32, bound: Bound, best: Move) {
        // Cache only if search was completed to avoid storing potentially invalid results
        if !self.load_terminator() {
            let entry = CacheEntry::new(self.board.hash(), depth, score, bound, best);
            self.cache.lock().unwrap().write(entry, self.board.ply);
        }
    }

    /// Returns `true` if the current node is the root node.
    fn root(&mut self) -> bool {
        self.board.ply == 0
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
