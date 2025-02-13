use crate::{
    parameters::*,
    tables::{Bound, Entry},
    types::{Move, MoveList, Score, MAX_PLY},
};

impl super::SearchThread<'_> {
    /// Performs an alpha-beta search in a fail-soft environment.
    pub fn search<const PV: bool, const ROOT: bool>(&mut self, mut alpha: i32, mut beta: i32, mut depth: i32) -> i32 {
        self.pv_table.clear(self.ply);

        // The search has been stopped by the UCI or the time control
        if self.should_interrupt_search() {
            return Score::ZERO;
        }

        if !ROOT {
            // Draw detection (50-move rule, threefold repetition)
            if self.board.is_draw() {
                // Use a little randomness to avoid 3-fold repetition blindness
                return -1 + (self.nodes.local() as i32 & 0x2);
            }

            // Mate Distance Pruning
            alpha = alpha.max(-Score::MATE + self.ply as i32);
            beta = beta.min(Score::MATE - (self.ply as i32) - 1);

            if alpha >= beta {
                return alpha;
            }
        }

        // Prevent overflows
        if self.ply >= MAX_PLY - 1 {
            return self.board.evaluate();
        }

        let in_check = self.board.in_check();

        // Quiescence search at the leaf nodes, skip if in check to avoid horizon effect
        if depth <= 0 && !in_check {
            return self.quiescence_search(alpha, beta);
        }

        depth = depth.max(0);

        // Update UCI statistics after the quiescence search to avoid counting the same node twice
        self.nodes.inc();
        self.sel_depth = self.sel_depth.max(self.ply);

        // Transposition table lookup and potential cutoff
        let entry = self.tt.read(self.board.hash(), self.ply);
        if let Some(entry) = entry {
            if !PV && should_cutoff(entry, alpha, beta, depth) {
                return entry.score;
            }
        }

        // Internal Iterative Reductions. If no hash move is found in the TT, reduce the search depth
        // to counter a potentially poor move ordering that could slow down the search on higher depths
        if entry.is_none() && depth >= iir_depth() {
            depth -= 1;
        }

        let eval = match entry {
            _ if in_check => -Score::INFINITY,
            Some(entry) => entry.score,
            None => self.board.evaluate() + self.corrhist.get(self.board),
        };

        self.killers[self.ply + 1] = Move::NULL;
        self.eval_stack[self.ply] = if in_check { -Score::INFINITY } else { eval };

        let improving = self.is_improving(in_check);

        if !ROOT && !PV && !in_check {
            // Reverse Futility Pruning
            if depth < rfp_depth() && eval - rfp_margin() * (depth - i32::from(improving)) > beta {
                return eval;
            }

            // Razoring
            if depth <= razoring_depth() && eval + razoring_margin() * depth + razoring_fixed_margin() < alpha {
                return self.quiescence_search(alpha, beta);
            }

            // Null Move Pruning
            if let Some(score) = self.null_move_pruning::<PV>(depth, beta, eval) {
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
        let mut quiets = MoveList::default();
        let mut captures = MoveList::default();
        let mut moves = self.board.generate_all_moves();
        let mut ordering = self.build_ordering(&moves, entry.map(|entry| entry.mv), 0);

        while let Some(mv) = moves.next(&mut ordering) {
            #[cfg(not(feature = "datagen"))]
            if !ROOT && moves_played > 0 && best_score > -Score::MATE_BOUND {
                // Futility Pruning. Leave the node since later moves with worse history
                // are unlikely to recover a score so far below alpha in very few moves.
                if !PV
                    && !in_check
                    && mv.is_quiet()
                    && depth <= fp_depth()
                    && eval + fp_margin() * depth + fp_fixed_margin() < alpha
                {
                    break;
                }

                // Late Move Pruning. Leave the node after trying enough quiet moves with no success.
                if mv.is_quiet()
                    && depth <= LMP_DEPTH
                    && quiets.len() as i32 > LMP_MARGIN + depth * depth / (2 - improving as i32)
                {
                    break;
                }

                // Static Exchange Evaluation Pruning. Skip moves that are losing material.
                if depth < see_depth()
                    && !self.board.see(mv, -[see_quiet_margin(), see_noisy_margin()][mv.is_capture() as usize] * depth)
                {
                    continue;
                }
            }

            let key_after = self.board.key_after(mv);
            self.tt.prefetch(key_after);

            if !self.apply_move(mv) {
                continue;
            }

            let nodes_before = self.nodes.local();

            let mut new_depth = depth - 1;
            let mut score = Score::NONE;

            if depth >= 3 && moves_played >= 3 {
                let r = self.calculate_reduction::<PV>(mv, depth, moves_played, improving, &entry);
                let d = new_depth - r;

                score = -self.search::<false, false>(-alpha - 1, -alpha, d);

                if score > alpha && r > 0 {
                    new_depth += i32::from(score > best_score + search_deeper_margin());

                    if new_depth > d {
                        score = -self.search::<false, false>(-alpha - 1, -alpha, new_depth);
                    }
                }
            } else if !PV || moves_played > 0 {
                score = -self.search::<false, false>(-alpha - 1, -alpha, new_depth);
            }

            if PV && (moves_played == 0 || score > alpha) {
                score = -self.search::<PV, false>(-beta, -alpha, new_depth);
            }

            self.revert_move();
            moves_played += 1;

            if ROOT {
                self.node_table.add(mv, self.nodes.local() - nodes_before);
            }

            // Early return to prevent processing potentially corrupted search results
            if self.stopped {
                return Score::ZERO;
            }

            if score > best_score {
                best_score = score;
                best_move = mv;

                if score > alpha {
                    alpha = score;
                    self.pv_table.update(self.ply, mv);
                }
            }

            if alpha >= beta {
                break;
            }

            if mv.is_quiet() {
                quiets.push(mv);
            } else {
                captures.push(mv);
            }
        }

        // Checkmate and stalemate detection
        if moves_played == 0 {
            return if in_check { Score::mated_in(self.ply) } else { Score::DRAW };
        }

        let bound = determine_bound(best_score, original_alpha, beta);
        if bound == Bound::Lower {
            self.update_ordering_heuristics(depth, best_move, captures.as_slice(), quiets.as_slice());
        }

        if !(in_check
            || best_move.is_capture()
            || (bound == Bound::Upper && best_score >= eval)
            || (bound == Bound::Lower && best_score <= eval))
        {
            self.corrhist.update(self.board, depth, best_score - eval);
        }

        self.tt.write(self.board.hash(), depth, best_score, bound, best_move, self.ply);
        best_score
    }

    fn is_improving(&self, in_check: bool) -> bool {
        let improving = || {
            let mut previous = self.eval_stack[self.ply - 2];
            if previous == -Score::INFINITY && self.ply >= 4 {
                previous = self.eval_stack[self.ply - 4];
            }
            self.eval_stack[self.ply] > previous
        };
        self.ply < 2 || (!in_check && improving())
    }

    /// If giving a free move to the opponent leads to a beta cutoff, it's highly likely
    /// to result in a cutoff after a real move is made, so the node can be pruned.
    pub fn null_move_pruning<const PV: bool>(&mut self, depth: i32, beta: i32, eval: i32) -> Option<i32> {
        if depth >= 4 && eval > beta && !self.board.is_last_move_null() && self.board.has_non_pawn_material() {
            let r = 3 + depth / 3 + ((eval - beta) / 200).min(4);

            self.apply_null_move();
            let score = -self.search::<PV, false>(-beta, -beta + 1, depth - r);
            self.revert_null_move();

            return match score {
                s if s >= Score::MATE_BOUND => Some(beta),
                s if s >= beta => Some(score),
                _ => None,
            };
        }
        None
    }

    /// Calculates the Late Move Reduction (LMR) for a given move.
    pub fn calculate_reduction<const PV: bool>(
        &self,
        mv: Move,
        depth: i32,
        moves: i32,
        improving: bool,
        entry: &Option<Entry>,
    ) -> i32 {
        fn to_f64(v: bool) -> f64 {
            i32::from(v) as f64
        }

        if !mv.is_quiet() {
            return 0;
        }

        // Fractional reductions
        let mut reduction = self.params.lmr(depth, moves);

        reduction -= self.history.get_main(!self.board.side_to_move(), mv) as f64 / lmr_history() as f64;

        reduction -= 0.88 * to_f64(PV);
        reduction -= 0.78 * to_f64(self.board.in_check());

        reduction += 0.91 * to_f64(entry.is_some_and(|e| e.mv.is_capture()));
        reduction += 0.48 * to_f64(improving);

        // Avoid negative reductions
        (reduction as i32).clamp(0, depth)
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
    fn update_ordering_heuristics(&mut self, depth: i32, best_move: Move, captures: &[Move], quiets: &[Move]) {
        if best_move.is_capture() {
            self.history.update_capture(self.board, best_move, captures, depth);
        } else {
            self.killers[self.ply] = best_move;
            self.history.update_main(self.board.side_to_move(), best_move, quiets, depth);
            self.history.update_continuation(self.board, best_move, quiets, depth);
        }
    }
}

const fn determine_bound(score: i32, alpha: i32, beta: i32) -> Bound {
    match score {
        s if s <= alpha => Bound::Upper,
        s if s >= beta => Bound::Lower,
        _ => Bound::Exact,
    }
}

const fn should_cutoff(entry: Entry, alpha: i32, beta: i32, depth: i32) -> bool {
    match entry.bound {
        _ if depth > entry.depth => false,
        Bound::Exact => true,
        Bound::Lower => entry.score >= beta,
        Bound::Upper => entry.score <= alpha,
    }
}
