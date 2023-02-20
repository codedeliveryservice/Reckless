use game::Score;

use super::{ordering, quiescence, SearchParams, SearchThread};

/// Implementation of minimax algorithm but instead of using two separate routines for the Min player
/// and the Max player, it passes on the negated score due to following mathematical relationship:
///
/// `max(a, b) == -min(-a, -b)`
///
/// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
pub fn negamax_search(mut p: SearchParams, thread: &mut SearchThread) -> Score {
    if thread.check_on() {
        return Score::INVALID;
    }

    thread.pv_length[p.ply] = p.ply;

    if p.ply > 0 && p.board.is_repetition() {
        return Score::ZERO;
    }

    if p.depth == 0 {
        return quiescence::quiescence_search(p, thread);
    }

    thread.nodes += 1;

    // Increase search depth if king is in check
    let in_check = p.board.is_in_check();
    if in_check {
        p.depth += 1;
    }

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

        // Perform a fail-hard beta cutoff
        if score >= p.beta {
            // The killer heuristic is intended only for ordering quiet moves
            if mv.is_quiet() {
                thread.killers.add(mv, p.ply);
            }

            return p.beta;
        }

        // Found a better move that maximizes alpha
        if score > p.alpha {
            p.alpha = score;

            thread.pv_table[p.ply][p.ply] = mv;

            for index in (p.ply + 1)..thread.pv_length[p.ply + 1] {
                thread.pv_table[p.ply][index] = thread.pv_table[p.ply + 1][index];
            }

            thread.pv_length[p.ply] = thread.pv_length[p.ply + 1];
        }
    }

    if legal_moves == 0 {
        return match in_check {
            // Since negamax evaluates positions from the point of view of the maximizing player,
            // we choose the longest path to checkmate by adding the depth (maximizing the score)
            true => Score::CHECKMATE + p.ply as i32,
            false => Score::STALEMATE,
        };
    }

    p.alpha
}
