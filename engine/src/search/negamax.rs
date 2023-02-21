use game::Score;

use super::{ordering, quiescence, SearchParams, SearchThread};

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

    // Static evaluation is unreliable when the king is under check,
    // so increase the search depth in this case
    let in_check = p.board.is_in_check();
    if in_check {
        p.depth += 1;
    }

    if p.depth == 0 {
        return quiescence::quiescence_search(p, thread);
    }

    thread.pv_length[p.ply] = p.ply;
    thread.nodes += 1;

    let mut legal_moves = 0;

    let moves = ordering::order_moves(&p, thread);
    for mv in moves {
        if p.board.make_move(mv).is_err() {
            continue;
        }

        legal_moves += 1;
        p.ply += 1;

        let child_params = SearchParams::new(p.board, -p.beta, -p.alpha, p.depth - 1, p.ply);
        let score = -negamax_search(child_params, thread);

        p.board.take_back();
        p.ply -= 1;

        // The opponent can force the score as low as beta, so if the move is "too good"
        // we perform a fail-high beta cutoff as the opponent is expected to avoid this position
        if score >= p.beta {
            // The killer heuristic is intended only for ordering quiet moves
            if mv.is_quiet() {
                thread.killers.add(mv, p.ply);
            }

            return p.beta;
        }

        // Found a better move that raises alpha closer to beta
        if score > p.alpha {
            p.alpha = score;
            thread.update_pv(p.ply, mv);
        }
    }

    if let Some(score) = is_game_over(legal_moves, in_check, &p) {
        return score;
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
