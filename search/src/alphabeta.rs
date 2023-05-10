use std::time::Instant;

use game::{Board, Move, Score, MAX_SEARCH_DEPTH};

use super::quiescence::QuiescenceSearch;
use super::{ordering::Ordering, CacheEntry, NodeKind, SearchParams, SearchThread};

pub struct AlphaBetaSearch<'a> {
    pub(crate) start_time: Instant,
    board: &'a mut Board,
    thread: &'a mut SearchThread,
    ply: usize,
}

impl<'a> AlphaBetaSearch<'a> {
    /// Creates a new `AlphaBetaSearch` instance.
    pub fn new(board: &'a mut Board, thread: &'a mut SearchThread) -> Self {
        Self {
            start_time: Instant::now(),
            board,
            thread,
            ply: Default::default(),
        }
    }

    /// Performs a search using alpha-beta pruning in a fail-hard environment.
    pub fn search(&mut self, mut p: SearchParams) -> Score {
        if let Some(score) = self.check_on() {
            return score;
        }
        if let Some(score) = self.detect_repetition() {
            return score;
        }
        if let Some(score) = self.validate_depth() {
            return score;
        }

        // Increase the search depth to avoid a horizon effect when evaluating the position
        let in_check = self.board.is_in_check();
        if in_check {
            p.depth += 1;
        }

        self.thread.nodes += 1;

        if let Some(score) = self.evaluate(&p) {
            return score;
        }
        if let Some(score) = self.read_cache_entry(&p) {
            return score;
        }
        if let Some(score) = self.null_move_pruning(&p, in_check) {
            return score;
        }

        // Values that are used to insert an entry into the cache
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::default();
        let mut kind = NodeKind::All;

        let mut moves_searched = 0;
        let pv_node = p.alpha != p.beta - 1;

        let mut ordering = Ordering::normal(self.board, self.ply, self.thread);
        while let Some(mv) = ordering.next() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            self.ply += 1;

            let score = if moves_searched == 0 {
                // Perform a full-width search on the first move
                -self.search(SearchParams::new(-p.beta, -p.alpha, p.depth - 1))
            } else {
                let late_search_stage = moves_searched >= 4 && p.depth >= 3;
                let simple_move = mv.is_quiet() && !mv.is_promotion();
                let is_lmr_applicable = late_search_stage && simple_move && !in_check && !pv_node;
                self.late_move_reduction(is_lmr_applicable, &p)
            };
            moves_searched += 1;

            self.ply -= 1;
            self.board.undo_move();

            // Update the TT entry information if the move is better than what we've found so far
            if score > best_score {
                best_score = score;
                best_move = mv;
            }

            // The move is too good for the opponent, so ignore an uninteresting branch and perform a beta cutoff
            if score >= p.beta {
                self.write_cache_entry(p.depth, score, NodeKind::Cut, mv);

                if mv.is_quiet() {
                    self.thread.killers.add(mv, self.ply);
                }

                return p.beta;
            }

            // The move raises the lower bound (a better move was found), so update the alpha value
            if score > p.alpha {
                p.alpha = score;
                kind = NodeKind::PV;

                if mv.is_quiet() {
                    self.thread.history.store(mv.start(), mv.target(), p.depth);
                }
            }
        }

        if let Some(score) = self.is_game_over(moves_searched > 0, in_check) {
            return score;
        }

        self.write_cache_entry(p.depth, best_score, kind, best_move);

        // Fail-low node (all moves were too good for the opponent)
        p.alpha
    }

    /// Returns the score of the current position if it's at the root node.
    fn evaluate(&mut self, p: &SearchParams) -> Option<Score> {
        if p.depth > 0 {
            return None;
        }
        Some(QuiescenceSearch::new(self.board, self.thread).search(p.alpha, p.beta, self.ply))
    }

    /// Returns the score of the current position if the search depth is too high.
    fn validate_depth(&mut self) -> Option<Score> {
        if self.ply > MAX_SEARCH_DEPTH - 1 {
            return Some(evaluation::evaluate_relative_score(self.board));
        }
        None
    }

    /// Returns a draw score if the current position is a repetition.
    fn detect_repetition(&mut self) -> Option<Score> {
        if self.ply > 0 && self.board.is_repetition() {
            return Some(Score::DRAW);
        }
        None
    }

    /// Checks if the search should be interrupted.
    #[inline(always)]
    fn check_on(&mut self) -> Option<Score> {
        if self.thread.nodes % 4096 != 0 || self.thread.current_depth < 2 {
            return None;
        }

        if self.thread.is_time_over() {
            self.thread.set_terminator(true);
        }

        self.thread.get_terminator().then_some(Score::INVALID)
    }

    /// Perform a reduced-depth search for an uninteresting move.
    fn late_move_reduction(&mut self, is_lmr_applicable: bool, p: &SearchParams) -> Score {
        if is_lmr_applicable {
            let lmr_score = -self.search(SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 2));
            // LMR assumes that the move is bad
            if lmr_score <= p.alpha {
                return lmr_score;
            }
        }

        // Fall back to a full-depth search if LMR failed
        self.principle_variation_search(p)
    }

    /// Performs a principle variation search with a closed window around alpha.
    #[inline(always)]
    fn principle_variation_search(&mut self, p: &SearchParams) -> Score {
        let score = -self.search(SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 1));

        let pv_mode_is_best = p.alpha >= score || score >= p.beta;
        if pv_mode_is_best {
            return score;
        }

        // Perform a normal search since our assumption was wrong
        -self.search(SearchParams::new(-p.beta, -p.alpha, p.depth - 1))
    }

    #[inline(always)]
    fn null_move_pruning(&mut self, p: &SearchParams, in_check: bool) -> Option<Score> {
        let can_prune = p.null_move_allowed && !in_check && p.depth >= 3;
        if !can_prune {
            return None;
        }

        self.board.make_null_move();
        self.ply += 1;

        let mut params = SearchParams::new(-p.beta, -p.beta + 1, p.depth - 3);
        params.null_move_allowed = false;
        let score = -self.search(params);

        self.ply -= 1;
        self.board.undo_null_move();

        (score >= p.beta).then_some(p.beta)
    }

    /// Reads a cache entry from the transposition table.
    #[inline(always)]
    fn read_cache_entry(&self, p: &SearchParams) -> Option<Score> {
        let entry = self.thread.cache.lock().unwrap().read(self.board.hash);
        entry.and_then(|entry| entry.get_score(p.alpha, p.beta, p.depth))
    }

    /// Writes a new cache entry to the transposition table.
    ///
    /// Caching is skipped if the search was interrupted, in which case the results of the
    /// search may be invalid and should not be cached.
    #[inline(always)]
    fn write_cache_entry(&mut self, depth: usize, score: Score, kind: NodeKind, best: Move) {
        if !self.thread.get_terminator() {
            let entry = CacheEntry::new(self.board.hash, depth, score, kind, best);
            self.thread.cache.lock().unwrap().write(entry);
        }
    }

    /// Returns `true` if the game is considered to be over either due to checkmate or stalemate.
    #[inline(always)]
    fn is_game_over(&self, legal_move_found: bool, in_check: bool) -> Option<Score> {
        if legal_move_found {
            return None;
        }

        match in_check {
            // Since negamax evaluates positions from the point of view of the maximizing player,
            // we choose the longest path to checkmate by adding the depth (maximizing the score)
            true => Some(-Score::CHECKMATE + self.ply as i32),
            false => Some(Score::DRAW),
        }
    }
}
