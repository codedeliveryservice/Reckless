use game::{Move, Score, Zobrist};

use super::{ordering::Ordering, quiescence, CacheEntry, NodeKind, SearchParams, SearchThread};

/// Performs a `negamax` search with alpha-beta pruning in a fail-hard environment.
///
/// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
pub fn negamax_search(mut p: SearchParams, thread: &mut SearchThread) -> Score {
    if thread.check_on() {
        return Score::INVALID;
    }

    if p.ply > 0 && p.board.is_repetition() {
        return Score::DRAW;
    }

    if p.ply > SearchParams::MAX_PLY - 1 {
        return quiescence::evaluate_statically(p.board);
    }

    // Static evaluation is unreliable when the king is under check,
    // so increase the search depth in this case
    let in_check = p.board.is_in_check();
    if in_check {
        p.depth += 1;
    }

    if p.depth == 0 {
        return quiescence::quiescence_search(p, thread);
    }

    thread.nodes += 1;

    // If the cache contains a relevant score, return it immediately
    let (tt_move, tt_score) = read_cache(&p, thread);
    if let Some(score) = tt_score {
        return score;
    }

    // Values that are used to insert an entry into the TT. An empty move should will never
    // enter the TT, since the score of making any first move is greater than negative
    // infinity, otherwise there're no legal moves and the search will return earlier
    let mut best_score = Score::NEGATIVE_INFINITY;
    let mut best_move = Move::default();
    let mut kind = NodeKind::All;

    let mut legal_moves = 0;

    let mut ordering = Ordering::generate(&p, thread, tt_move);
    while let Some(mv) = ordering.next() {
        if p.board.make_move(mv).is_err() {
            continue;
        }

        legal_moves += 1;

        let child_params = SearchParams::new(p.board, -p.beta, -p.alpha, p.depth - 1, p.ply + 1);
        let score = -negamax_search(child_params, thread);

        p.board.take_back();

        // Update the TT entry information if the move is better than what we've found so far
        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        // The move is too good for the opponent which makes the position not interesting for us,
        // as we're expected to avoid a bad position, so we can perform a beta cutoff
        if score >= p.beta {
            let entry = CacheEntry::new(p.board.hash_key, p.depth, score, NodeKind::Cut, mv);
            write_cache_entry(entry, thread);

            // The killer heuristic is intended only for ordering quiet moves
            if mv.is_quiet() {
                thread.killers.add(mv, p.ply);
            }

            return p.beta;
        }

        // Found a better move that raises alpha
        if score > p.alpha {
            p.alpha = score;
            kind = NodeKind::PV;
        }
    }

    if let Some(score) = is_game_over(legal_moves, in_check, &p) {
        return score;
    }

    let entry = CacheEntry::new(p.board.hash_key, p.depth, best_score, kind, best_move);
    write_cache_entry(entry, thread);

    // The variation is useless, so it's a fail-low node
    p.alpha
}

#[inline(always)]
fn read_cache(p: &SearchParams, thread: &SearchThread) -> (Option<Move>, Option<Score>) {
    match read_cache_entry(p.board.hash_key, thread) {
        Some(entry) => (Some(entry.best), entry.get_score(&p)),
        _ => (None, None),
    }
}

#[inline(always)]
fn read_cache_entry(hash: Zobrist, thread: &SearchThread) -> Option<CacheEntry> {
    thread.cache.lock().unwrap().read(hash)
}

#[inline(always)]
fn write_cache_entry(entry: CacheEntry, thread: &mut SearchThread) {
    // Caching when search has been aborted will result in invalid data in the TT
    if !thread.tc.is_time_over() && !thread.requested_termination() {
        thread.cache.lock().unwrap().write(entry);
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
        true => Some(Score::CHECKMATE + p.ply as i32),
        false => Some(Score::DRAW),
    }
}
