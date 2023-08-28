use game::{Board, Move, Score};

use super::SearchThread;
use crate::{heuristics::*, CacheEntry, NodeKind, ALPHABETA_STAGES};

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
        if let Some(score) = self.is_draw() {
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

        let (cache_score, cache_move) = self.read_cache_entry(alpha, beta, depth);
        if let Some(score) = cache_score {
            return score;
        }

        if let Some(score) = self.null_move_pruning(beta, depth) {
            return score;
        }

        self.search_moves(alpha, beta, depth, cache_move)
    }

    fn search_moves(&mut self, mut alpha: Score, beta: Score, depth: usize, cache_move: Option<Move>) -> Score {
        let mut best_score = -Score::INFINITY;
        let mut best_move = None;
        let mut kind = NodeKind::All;

        let mut move_index = 0;
        let mut moves = self.board.generate_moves();
        let mut ordering = self.build_ordering(ALPHABETA_STAGES, &moves, cache_move);

        while let Some(mv) = moves.next(&mut ordering) {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            let reduction = self.calculate_reduction(mv, move_index, depth);
            let score = self.calculate_score(alpha, beta, depth, reduction, move_index);

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
    fn calculate_score(&mut self, alpha: Score, beta: Score, depth: usize, reduction: usize, move_index: usize) -> Score {
        if move_index == 0 {
            return -self.search(-beta, -alpha, depth - 1);
        }

        // Null window search with possible late move reduction
        let mut score = -self.search(-alpha - 1, -alpha, depth - reduction);

        // If the search fails and reduction applied, re-search with full depth
        if alpha < score && reduction > 1 {
            score = -self.search(-alpha - 1, -alpha, depth - 1);
        }

        // If the search fails again, proceed to a full window search with full depth
        if alpha < score && score < beta {
            score = -self.search(-beta, -alpha, depth - 1);
        }

        score
    }

    /// Calculates the reduction to be applied to the current move.
    fn calculate_reduction(&mut self, mv: Move, move_index: usize, depth: usize) -> usize {
        let tactical = mv.is_capture() || mv.is_promotion() || self.board.is_in_check();
        let can_reduce = move_index >= 4 && depth >= 3 && !tactical;
        let reduction = if can_reduce { 3 } else { 1 };
        reduction
    }

    /// Returns the score of the current position if it's at the root node.
    fn evaluate(&mut self, alpha: Score, beta: Score, depth: usize) -> Option<Score> {
        if depth == 0 {
            return Some(self.quiescence_search(alpha, beta));
        }
        None
    }

    /// Returns a draw score if the current position is a draw by repetition or fifty-move rule.
    fn is_draw(&mut self) -> Option<Score> {
        if self.board.ply > 0 && (self.board.is_repetition() || self.board.is_fifty_move_draw()) {
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
    fn read_cache_entry(&self, alpha: Score, beta: Score, depth: usize) -> (Option<Score>, Option<Move>) {
        if let Some(entry) = self.thread.cache.lock().unwrap().read(self.board.hash, self.board.ply) {
            if entry.depth < depth as u8 {
                return (None, Some(entry.best));
            }

            let score = match entry.kind {
                NodeKind::PV => Some(entry.score),
                NodeKind::Cut if entry.score >= beta => Some(beta),
                NodeKind::All if entry.score <= alpha => Some(alpha),
                _ => None,
            };
            return (score, Some(entry.best));
        }
        (None, None)
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
