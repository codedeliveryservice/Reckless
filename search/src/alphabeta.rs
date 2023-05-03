use std::time::Instant;

use game::{Board, Move, Score, MAX_SEARCH_DEPTH};

use super::quiescence::{self, QuiescenceSearch};
use super::{ordering::Ordering, CacheEntry, NodeKind, SearchParams, SearchThread};

pub struct AlphaBetaSearch<'a> {
    pub(crate) start_time: Instant,
    board: &'a mut Board,
    thread: &'a mut SearchThread,
}

impl<'a> AlphaBetaSearch<'a> {
    /// Creates a new `AlphaBetaSearch` instance.
    pub fn new(board: &'a mut Board, thread: &'a mut SearchThread) -> Self {
        Self {
            start_time: Instant::now(),
            board,
            thread,
        }
    }

    /// Performs a search using alpha-beta pruning in a fail-hard environment.
    pub fn search(&mut self, mut p: SearchParams) -> Score {
        if let Some(value) = self.check_on() {
            return value;
        }

        let repetition = p.ply > 0 && self.board.is_repetition();
        if repetition {
            return Score::DRAW;
        }

        let max_depth_reached = p.ply > MAX_SEARCH_DEPTH - 1;
        if max_depth_reached {
            return quiescence::evaluate_statically(self.board);
        }

        // Static evaluation is unreliable when the king is under check
        let in_check = self.board.is_in_check();
        if in_check {
            p.depth += 1;
        }

        let root_node = p.depth == 0;
        if root_node {
            return QuiescenceSearch::new(self.board, self.thread).search(p.alpha, p.beta, p.ply);
        }

        self.thread.nodes += 1;

        if let Some(score) = self.read_cache_entry(&p) {
            return score;
        }

        if let Some(score) = self.null_move_pruning(&mut p, in_check) {
            return score;
        }

        // Values that are used to insert an entry into the cache
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::default();
        let mut kind = NodeKind::All;

        let mut legal_move_found = false;
        let mut pv_found = false;

        let mut ordering = Ordering::normal(self.board, p.ply, self.thread);
        while let Some(mv) = ordering.next() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            legal_move_found = true;

            let score = match pv_found {
                true => self.dive_pvs(&mut p),
                false => self.dive_normal(&mut p),
            };

            self.board.undo_move();

            // Update the TT entry information if the move is better than what we've found so far
            if score > best_score {
                best_score = score;
                best_move = mv;
            }

            // The move is too good for the opponent which makes the position not interesting for us,
            // as we're expected to avoid a bad position, so we can perform a beta cutoff
            if score >= p.beta {
                let entry = CacheEntry::new(self.board.hash, p.depth, score, NodeKind::Cut, mv);
                self.write_cache_entry(entry);

                // The killer heuristic is intended only for ordering quiet moves
                if mv.is_quiet() {
                    self.thread.killers.add(mv, p.ply);
                }

                return p.beta;
            }

            // Found a better move that raises alpha
            if score > p.alpha {
                p.alpha = score;
                kind = NodeKind::PV;
                pv_found = true;

                // The history heuristic is intended only for ordering quiet moves
                if mv.is_quiet() {
                    self.thread.history.store(mv.start(), mv.target(), p.depth);
                }
            }
        }

        if let Some(score) = self.is_game_over(legal_move_found, in_check, p.ply) {
            return score;
        }

        let entry = CacheEntry::new(self.board.hash, p.depth, best_score, kind, best_move);
        self.write_cache_entry(entry);

        // The variation is useless, so it's a fail-low node
        p.alpha
    }

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
    fn dive_normal(&mut self, p: &mut SearchParams) -> Score {
        let params = SearchParams::new(-p.beta, -p.alpha, p.depth - 1, p.ply + 1);
        -self.search(params)
    }

    #[inline(always)]
    fn dive_pvs(&mut self, p: &mut SearchParams) -> Score {
        // Search with a closed window around alpha
        let params = SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 1, p.ply + 1);
        let score = -self.search(params);

        // Prove that any other move is worse than the PV move we've already found
        if p.alpha >= score || score >= p.beta {
            return score;
        }

        // Perform a normal search if we find that our assumption was wrong
        self.dive_normal(p)
    }

    #[inline(always)]
    fn null_move_pruning(&mut self, p: &mut SearchParams, in_check: bool) -> Option<Score> {
        if p.depth >= 3 && p.allow_nmp && !in_check {
            self.board.make_null_move();

            let mut params = SearchParams::new(-p.beta, -p.beta + 1, p.depth - 3, p.ply + 1);
            params.allow_nmp = false;

            let score = -self.search(params);
            self.board.undo_null_move();

            if score >= p.beta {
                return Some(p.beta);
            }
        }
        None
    }

    #[inline(always)]
    fn read_cache_entry(&self, p: &SearchParams) -> Option<Score> {
        let entry = self.thread.cache.lock().unwrap().read(self.board.hash);
        entry.and_then(|entry| entry.get_score(p.alpha, p.beta, p.depth))
    }

    #[inline(always)]
    fn write_cache_entry(&mut self, entry: CacheEntry) {
        // Caching on an interrupted search will result in invalid results being written
        if !self.thread.get_terminator() {
            self.thread.cache.lock().unwrap().write(entry);
        }
    }

    /// Returns `true` if the game is considered to be over either due to checkmate or stalemate.
    #[inline(always)]
    fn is_game_over(&self, legal_move_found: bool, in_check: bool, ply: usize) -> Option<Score> {
        if legal_move_found {
            return None;
        }

        match in_check {
            // Since negamax evaluates positions from the point of view of the maximizing player,
            // we choose the longest path to checkmate by adding the depth (maximizing the score)
            true => Some(-Score::CHECKMATE + ply as i32),
            false => Some(Score::DRAW),
        }
    }
}
