use game::{Board, Move, Score, Zobrist, MAX_SEARCH_DEPTH};

use crate::quiescence::QuiescenceSearch;

use super::{ordering::Ordering, quiescence, CacheEntry, NodeKind, SearchParams, SearchThread};

pub struct AlphaBetaSearch<'a> {
    board: &'a mut Board,
    thread: &'a mut SearchThread,
}

impl<'a> AlphaBetaSearch<'a> {
    /// Creates a new `AlphaBetaSearch` instance.
    pub fn new(board: &'a mut Board, thread: &'a mut SearchThread) -> Self {
        Self { board, thread }
    }

    /// Performs a `negamax` search with alpha-beta pruning in a fail-hard environment.
    ///
    /// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
    pub fn negamax_search(&mut self, mut p: SearchParams) -> Score {
        if self.thread.nodes % 4096 == 0 {
            if self.thread.is_time_over() {
                self.thread.set_terminator(true);
            }

            if self.thread.get_terminator() {
                return Score::INVALID;
            }
        }

        if p.ply > 0 && self.board.is_repetition() {
            return Score::DRAW;
        }

        if p.ply > MAX_SEARCH_DEPTH - 1 {
            return quiescence::evaluate_statically(self.board);
        }

        // Static evaluation is unreliable when the king is under check,
        // so increase the search depth in this case
        let in_check = self.board.is_in_check();
        if in_check {
            p.depth += 1;
        }

        if p.depth == 0 {
            return QuiescenceSearch::new(self.board, self.thread).search(p.alpha, p.beta, p.ply);
        }

        self.thread.nodes += 1;

        // If the cache contains a relevant score, return it immediately
        if let Some(score) = self.read_cache(&p) {
            return score;
        }

        if p.depth >= 3 && p.allow_nmp && !in_check {
            let score = self.null_move_pruning(&mut p);
            if score >= p.beta {
                return p.beta;
            }
        }

        // Values that are used to insert an entry into the TT. An empty move should will never
        // enter the TT, since the score of making any first move is greater than negative
        // infinity, otherwise there're no legal moves and the search will return earlier
        let mut best_score = -Score::INFINITY;
        let mut best_move = Move::default();
        let mut kind = NodeKind::All;

        let mut legal_moves = 0;
        let mut pv_found = false;

        let mut ordering = Ordering::normal(self.board, p.ply, self.thread);
        while let Some(mv) = ordering.next() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            legal_moves += 1;

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
                let entry = CacheEntry::new(self.board.hash_key, p.depth, score, NodeKind::Cut, mv);
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

        if let Some(score) = Self::is_game_over(legal_moves, in_check, &p) {
            return score;
        }

        let entry = CacheEntry::new(self.board.hash_key, p.depth, best_score, kind, best_move);
        self.write_cache_entry(entry);

        // The variation is useless, so it's a fail-low node
        p.alpha
    }

    #[inline(always)]
    fn dive_normal(&mut self, p: &mut SearchParams) -> Score {
        let params = SearchParams::new(-p.beta, -p.alpha, p.depth - 1, p.ply + 1);
        -self.negamax_search(params)
    }

    #[inline(always)]
    fn dive_pvs(&mut self, p: &mut SearchParams) -> Score {
        // Search with a closed window around alpha
        let params = SearchParams::new(-p.alpha - 1, -p.alpha, p.depth - 1, p.ply + 1);
        let score = -self.negamax_search(params);

        // Prove that any other move is worse than the PV move we've already found
        if p.alpha >= score || score >= p.beta {
            return score;
        }

        // Perform a normal search if we find that our assumption was wrong
        self.dive_normal(p)
    }

    fn null_move_pruning(&mut self, p: &mut SearchParams) -> Score {
        self.board.make_null_move();

        let mut params = SearchParams::new(-p.beta, -p.beta + 1, p.depth - 3, p.ply + 1);
        params.allow_nmp = false;

        let score = -self.negamax_search(params);
        self.board.undo_null_move();

        score
    }

    #[inline(always)]
    fn read_cache(&self, p: &SearchParams) -> Option<Score> {
        match self.read_cache_entry(self.board.hash_key) {
            Some(entry) => entry.get_score(p.alpha, p.beta, p.depth),
            _ => None,
        }
    }

    #[inline(always)]
    fn read_cache_entry(&self, hash: Zobrist) -> Option<CacheEntry> {
        self.thread.cache.lock().unwrap().read(hash)
    }

    #[inline(always)]
    fn write_cache_entry(&mut self, entry: CacheEntry) {
        // Caching when search has been aborted will result in invalid data in the TT
        if !self.thread.is_time_over() && !self.thread.get_terminator() {
            self.thread.cache.lock().unwrap().write(entry);
        }
    }

    /// Returns `true` if the game is considered to be over either due to checkmate or stalemate.
    #[inline(always)]
    fn is_game_over(legal_moves: i32, in_check: bool, p: &SearchParams) -> Option<Score> {
        if legal_moves > 0 {
            return None;
        }

        match in_check {
            // Since negamax evaluates positions from the point of view of the maximizing player,
            // we choose the longest path to checkmate by adding the depth (maximizing the score)
            true => Some(-Score::CHECKMATE + p.ply as i32),
            false => Some(Score::DRAW),
        }
    }
}
