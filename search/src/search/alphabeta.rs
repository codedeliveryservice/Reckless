use game::{Board, Move, Score, MAX_SEARCH_DEPTH};

use super::{SearchParams, SearchThread};
use crate::{heuristics::*, CacheEntry, NodeKind};

/// Implementation of the negamax algorithm with alpha-beta pruning.
pub struct AlphaBetaSearch<'a> {
    pub(super) board: &'a mut Board,
    pub(super) thread: &'a mut SearchThread,
    pub(super) ply: usize,
    pub(super) killers: KillerMoves,
    pub(super) history: HistoryMoves,
}

impl<'a> AlphaBetaSearch<'a> {
    /// Creates a new `AlphaBetaSearch` instance.
    pub fn new(board: &'a mut Board, thread: &'a mut SearchThread) -> Self {
        Self {
            board,
            thread,
            ply: Default::default(),
            killers: Default::default(),
            history: Default::default(),
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

        self.search_moves(&mut p, in_check)
    }

    fn search_moves(&mut self, p: &mut SearchParams, in_check: bool) -> Score {
        let pv_node = p.alpha != p.beta - 1;

        let mut best_score = -Score::INFINITY;
        let mut best_move = None;
        let mut kind = NodeKind::All;

        let mut move_index = 0;
        let mut ordering = self.build_normal_ordering();

        while let Some(mv) = ordering.next() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            self.ply += 1;

            let score = self.calculate_score(p, mv, move_index, in_check, pv_node);

            self.ply -= 1;
            self.board.undo_move();

            move_index += 1;

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }

            if score > p.alpha {
                p.alpha = score;
                kind = NodeKind::PV;
            }

            if score >= p.beta {
                if mv.is_quiet() {
                    self.killers.add(mv, self.ply);
                    self.history.store(mv, p.depth);
                }

                self.write_cache_entry(p.depth, score, NodeKind::Cut, mv);
                return p.beta;
            }
        }

        if let Some(score) = self.is_game_over(best_move.is_some(), in_check) {
            return score;
        }

        self.write_cache_entry(p.depth, best_score, kind, best_move.unwrap());
        best_score
    }

    /// Calculates the score of the current position.
    fn calculate_score(
        &mut self,
        p: &SearchParams,
        mv: Move,
        move_index: usize,
        in_check: bool,
        pv_node: bool,
    ) -> Score {
        if move_index == 0 {
            return -self.search(SearchParams::new(-p.beta, -p.alpha, p.depth - 1));
        }

        let tactical_move = mv.is_capture() || mv.is_promotion();

        if !in_check && !pv_node && !tactical_move {
            if let Some(score) = self.late_move_reduction(p, move_index) {
                return score;
            }
        }

        // Fall back to a full-depth search
        self.principle_variation_search(p)
    }

    /// Returns the score of the current position if it's at the root node.
    fn evaluate(&mut self, p: &SearchParams) -> Option<Score> {
        if p.depth == 0 {
            return Some(self.quiescence_search(p.alpha, p.beta, self.ply));
        }
        None
    }

    /// Returns the score of the current position if the search depth is too high.
    fn validate_depth(&mut self) -> Option<Score> {
        if self.ply >= MAX_SEARCH_DEPTH {
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
    fn late_move_reduction(&mut self, p: &SearchParams, move_index: usize) -> Option<Score> {
        if move_index >= 4 && p.depth >= 3 {
            let lmr_score = -self.search(SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 2));
            return (lmr_score <= p.alpha).then_some(lmr_score);
        }
        None
    }

    /// Performs a principle variation search with a closed window around alpha.
    #[inline(always)]
    fn principle_variation_search(&mut self, p: &SearchParams) -> Score {
        let score = -self.search(SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 1));

        let is_pv_move_better = p.alpha >= score || score >= p.beta;
        if is_pv_move_better {
            return score;
        }

        // Perform a normal search since our assumption was wrong
        -self.search(SearchParams::new(-p.beta, -p.alpha, p.depth - 1))
    }

    #[inline(always)]
    fn null_move_pruning(&mut self, p: &SearchParams, in_check: bool) -> Option<Score> {
        let can_prune = !self.board.is_last_move_null() && !in_check && p.depth >= 3;
        if !can_prune {
            return None;
        }

        self.board.make_null_move();
        self.ply += 1;

        let score = -self.search(SearchParams::new(-p.beta, -p.beta + 1, p.depth - 3));

        self.ply -= 1;
        self.board.undo_null_move();

        (score >= p.beta).then_some(p.beta)
    }

    /// Reads a cache entry from the transposition table.
    #[inline(always)]
    fn read_cache_entry(&self, p: &SearchParams) -> Option<Score> {
        let cache = self.thread.cache.lock().unwrap();
        let entry = cache.read(self.board.hash, self.ply);
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
            self.thread.cache.lock().unwrap().write(entry, self.ply);
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