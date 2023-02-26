use game::Score;

use super::{ordering::Ordering, quiescence, CacheEntry, NodeKind, SearchParams, SearchThread};

/// Performs a `negamax` search with alpha-beta pruning in a fail-hard environment.
///
/// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
pub fn negamax_search(mut p: SearchParams, thread: &mut SearchThread) -> Score {
    if thread.check_on() {
        return Score::INVALID;
    }

    if p.ply > 0 && p.board.is_repetition() {
        return Score::ZERO;
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

    if let Some(entry) = thread.cache.lock().unwrap().read(p.board.hash_key) {
        if let Some(score) = entry.get_score(&p) {
            return score;
        }
    }

    let mut best_score = Score::NEGATIVE_INFINITY;
    let mut best_move = Default::default();
    let mut kind = NodeKind::All;

    let mut legal_moves = 0;

    let mut ordering = Ordering::generate(&p, thread);
    while let Some(mv) = ordering.next() {
        if p.board.make_move(mv).is_err() {
            continue;
        }

        legal_moves += 1;

        let child_params = SearchParams::new(p.board, -p.beta, -p.alpha, p.depth - 1, p.ply + 1);
        let score = -negamax_search(child_params, thread);

        p.board.take_back();

        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        // The opponent can force the score as low as beta, so if the move is "too good"
        // we perform a fail-high beta cutoff as the opponent is expected to avoid this position
        if score >= p.beta {
            if !thread.tc.is_time_over() && !thread.requested_termination() {
                let entry = CacheEntry::new(p.board.hash_key, p.depth, score, NodeKind::Cut, mv);
                thread.cache.lock().unwrap().write(entry);
            }

            // The killer heuristic is intended only for ordering quiet moves
            if mv.is_quiet() {
                thread.killers.add(mv, p.ply);
            }

            return p.beta;
        }

        // Found a better move that raises alpha closer to beta
        if score > p.alpha {
            p.alpha = score;
            kind = NodeKind::PV;
        }
    }

    if let Some(score) = is_game_over(legal_moves, in_check, &p) {
        return score;
    }

    if !thread.tc.is_time_over() && !thread.requested_termination() {
        let entry = CacheEntry::new(p.board.hash_key, p.depth, best_score, kind, best_move);
        thread.cache.lock().unwrap().write(entry);
    }

    // The variation is useless, so it's a fail-low node
    p.alpha
}

/// Returns `true` if the game is considered to be over either due to checkmate or stalemate.
#[inline(always)]
fn is_game_over(legal_moves: i32, in_check: bool, p: &SearchParams) -> Option<Score> {
    if legal_moves > 0 {
        return None;
    }

    Some(match in_check {
        // Since negamax evaluates positions from the point of view of the maximizing player,
        // we choose the longest path to checkmate by adding the depth (maximizing the score)
        true => Score::CHECKMATE + p.ply as i32,
        false => Score::STALEMATE,
    })
}
