use game::{Move, Score, Zobrist, MAX_SEARCH_DEPTH};

use super::{ordering::Ordering, quiescence, CacheEntry, NodeKind, SearchParams, SearchThread};

/// Performs a `negamax` search with alpha-beta pruning in a fail-hard environment.
///
/// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
pub fn negamax_search(mut p: SearchParams, thread: &mut SearchThread) -> Score {
    if thread.nodes % 4096 == 0 {
        if thread.is_time_over() {
            thread.set_terminator(true);
        }

        if thread.get_terminator() {
            return Score::INVALID;
        }
    }

    if p.ply > 0 && p.board.is_repetition() {
        return Score::DRAW;
    }

    if p.ply > MAX_SEARCH_DEPTH - 1 {
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
    if let Some(score) = read_cache(&p, thread) {
        return score;
    }

    if p.depth >= 3 && p.allow_nmp && !in_check {
        let score = null_move_pruning(&mut p, thread);
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

    let mut ordering = Ordering::normal(&p, thread);
    while let Some(mv) = ordering.next() {
        if p.board.make_move(mv).is_err() {
            continue;
        }

        legal_moves += 1;

        let score = match pv_found {
            true => dive_pvs(&mut p, thread),
            false => dive_normal(&mut p, thread),
        };

        p.board.undo_move();

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
            pv_found = true;

            // The history heuristic is intended only for ordering quiet moves
            if mv.is_quiet() {
                thread.history.store(mv.start(), mv.target(), p.depth);
            }
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
fn dive_normal(p: &mut SearchParams, thread: &mut SearchThread) -> Score {
    let params = SearchParams::new(p.board, -p.beta, -p.alpha, p.depth - 1, p.ply + 1);
    -negamax_search(params, thread)
}

#[inline(always)]
fn dive_pvs(p: &mut SearchParams, thread: &mut SearchThread) -> Score {
    // Search with a closed window around alpha
    let params = SearchParams::new(p.board, -p.alpha - 1, -p.alpha, p.depth - 1, p.ply + 1);
    let score = -negamax_search(params, thread);

    // Prove that any other move is worse than the PV move we've already found
    if p.alpha >= score || score >= p.beta {
        return score;
    }

    // Perform a normal search if we find that our assumption was wrong
    dive_normal(p, thread)
}

fn null_move_pruning(p: &mut SearchParams, thread: &mut SearchThread) -> Score {
    p.board.make_null_move();

    let mut params = SearchParams::new(p.board, -p.beta, -p.beta + 1, p.depth - 3, p.ply + 1);
    params.allow_nmp = false;

    let score = -negamax_search(params, thread);
    p.board.undo_null_move();

    score
}

#[inline(always)]
fn read_cache(p: &SearchParams, thread: &SearchThread) -> Option<Score> {
    match read_cache_entry(p.board.hash_key, thread) {
        Some(entry) => entry.get_score(p.alpha, p.beta, p.depth),
        _ => None,
    }
}

#[inline(always)]
fn read_cache_entry(hash: Zobrist, thread: &SearchThread) -> Option<CacheEntry> {
    thread.cache.lock().unwrap().read(hash)
}

#[inline(always)]
fn write_cache_entry(entry: CacheEntry, thread: &mut SearchThread) {
    // Caching when search has been aborted will result in invalid data in the TT
    if !thread.is_time_over() && !thread.get_terminator() {
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
        true => Some(-Score::CHECKMATE + p.ply as i32),
        false => Some(Score::DRAW),
    }
}
