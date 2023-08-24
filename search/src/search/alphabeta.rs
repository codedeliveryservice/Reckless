use game::{Board, Move, Score};

use super::SearchThread;
use crate::{heuristics::*, CacheEntry, NodeKind};

/// Implementation of the negamax algorithm with alpha-beta pruning.
pub struct AlphaBetaSearch<'a> {
    pub(super) board: &'a mut Board,
    pub(super) thread: &'a mut SearchThread,
    pub(super) killers: KillerMoves,
    pub(super) history: HistoryMoves,
}

impl<'a> AlphaBetaSearch<'a> {
    /// Creates a new `AlphaBetaSearch` instance.
    pub fn new(board: &'a mut Board, thread: &'a mut SearchThread) -> Self {
        Self {
            board,
            thread,
            killers: Default::default(),
            history: Default::default(),
        }
    }

    /// Performs a search using alpha-beta pruning in a fail-hard environment.
    pub fn search(&mut self, alpha: Score, beta: Score, mut depth: usize) -> Score {
        if let Some(score) = self.check_on() {
            return score;
        }
        if let Some(score) = self.detect_repetition() {
            return score;
        }

        // Check extension
        if self.board.is_in_check() {
            depth += 1;
        }

        self.thread.nodes += 1;

        if let Some(score) = self.evaluate(alpha, beta, depth) {
            return score;
        }
        if let Some(score) = self.read_cache_entry(alpha, beta, depth) {
            return score;
        }
        if let Some(score) = self.null_move_pruning(beta, depth) {
            return score;
        }

        self.search_moves(alpha, beta, depth)
    }

    fn search_moves(&mut self, mut alpha: Score, beta: Score, depth: usize) -> Score {
        let pv_node = alpha != beta - 1;

        let mut best_score = -Score::INFINITY;
        let mut best_move = None;
        let mut kind = NodeKind::All;

        let mut move_index = 0;
        let mut ordering = self.build_normal_ordering();

        while let Some(mv) = ordering.next() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            let score = self.calculate_score(alpha, beta, depth, mv, move_index, pv_node);
            self.board.undo_move();

            move_index += 1;

            if score > best_score {
                best_score = score;
                best_move = Some(mv);
            }

            if score > alpha {
                alpha = score;
                kind = NodeKind::PV;
            }

            if score >= beta {
                if mv.is_quiet() {
                    self.killers.add(mv, self.board.ply);
                    self.history.store(mv, depth);
                }

                self.write_cache_entry(depth, score, NodeKind::Cut, mv);
                return beta;
            }
        }

        if let Some(score) = self.is_game_over(best_move.is_some()) {
            return score;
        }

        self.write_cache_entry(depth, best_score, kind, best_move.unwrap());
        best_score
    }

    /// Calculates the score of the current position.
    fn calculate_score(
        &mut self,
        alpha: Score,
        beta: Score,
        depth: usize,
        mv: Move,
        move_index: usize,
        pv_node: bool,
    ) -> Score {
        if move_index == 0 {
            return -self.search(-beta, -alpha, depth - 1);
        }

        let tactical_move = mv.is_capture() || mv.is_promotion();

        if !pv_node && !tactical_move && !self.board.is_in_check() {
            if let Some(score) = self.late_move_reduction(alpha, depth, move_index) {
                return score;
            }
        }

        // Fall back to a full-depth search
        self.principle_variation_search(alpha, beta, depth)
    }

    /// Returns the score of the current position if it's at the root node.
    fn evaluate(&mut self, alpha: Score, beta: Score, depth: usize) -> Option<Score> {
        if depth == 0 {
            return Some(self.quiescence_search(alpha, beta));
        }
        None
    }

    /// Returns a draw score if the current position is a repetition.
    fn detect_repetition(&mut self) -> Option<Score> {
        if self.board.ply > 0 && self.board.is_repetition() {
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
    fn late_move_reduction(&mut self, alpha: Score, depth: usize, move_index: usize) -> Option<Score> {
        if move_index >= 4 && depth >= 3 {
            let lmr_score = -self.search(-alpha - 1, -alpha, depth - 2);
            return (lmr_score <= alpha).then_some(lmr_score);
        }
        None
    }

    /// Performs a principle variation search with a closed window around alpha.
    #[inline(always)]
    fn principle_variation_search(&mut self, alpha: Score, beta: Score, depth: usize) -> Score {
        let score = -self.search(-alpha - 1, -alpha, depth - 1);

        let is_pv_move_better = alpha >= score || score >= beta;
        if is_pv_move_better {
            return score;
        }

        // Perform a normal search since our assumption was wrong
        -self.search(-beta, -alpha, depth - 1)
    }

    #[inline(always)]
    fn null_move_pruning(&mut self, beta: Score, depth: usize) -> Option<Score> {
        let can_prune = !self.board.is_last_move_null() && !self.board.is_in_check() && depth >= 3;
        if !can_prune {
            return None;
        }

        self.board.make_null_move();
        let score = -self.search(-beta, -beta + 1, depth - 3);
        self.board.undo_null_move();

        (score >= beta).then_some(beta)
    }

    /// Reads a cache entry from the transposition table.
    #[inline(always)]
    fn read_cache_entry(&self, alpha: Score, beta: Score, depth: usize) -> Option<Score> {
        let cache = self.thread.cache.lock().unwrap();
        let entry = cache.read(self.board.hash, self.board.ply);
        entry.and_then(|entry| entry.get_score(alpha, beta, depth))
    }

    /// Writes a new cache entry to the transposition table.
    ///
    /// Caching is skipped if the search was interrupted, in which case the results of the
    /// search may be invalid and should not be cached.
    #[inline(always)]
    fn write_cache_entry(&mut self, depth: usize, score: Score, kind: NodeKind, best: Move) {
        if !self.thread.get_terminator() {
            let entry = CacheEntry::new(self.board.hash, depth, score, kind, best);
            self.thread.cache.lock().unwrap().write(entry, self.board.ply);
        }
    }

    /// Returns `true` if the game is considered to be over either due to checkmate or stalemate.
    #[inline(always)]
    fn is_game_over(&mut self, legal_move_found: bool) -> Option<Score> {
        if legal_move_found {
            return None;
        }

        match self.board.is_in_check() {
            // Since negamax evaluates positions from the point of view of the maximizing player,
            // we choose the longest path to checkmate by adding the depth (maximizing the score)
            true => Some(-Score::CHECKMATE + self.board.ply as i32),
            false => Some(Score::DRAW),
        }
    }
}
