use game::{Board, Color, Score};

use super::{ordering, SearchParams, SearchThread};
use crate::evaluation;

/// `Quiescence Search` performs a `negamax` search from the root node until the position
/// becomes stable to evaluate it statically. This minimizes the horizon effect for volatile
/// positions when threads and opportunities that go beyond the fixed depth of the search
/// will remain undetected.
///
/// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
/// for more information.
pub fn quiescence_search(mut p: SearchParams, thread: &mut SearchThread) -> Score {
    if thread.check_on() {
        return Score::INVALID;
    }

    thread.nodes += 1;

    let evaluation = evaluate_statically(p.board);

    if evaluation >= p.beta {
        return p.beta;
    }

    if evaluation > p.alpha {
        p.alpha = evaluation;
    }

    let moves = ordering::order_moves(&p, thread);
    for mv in moves {
        if !mv.is_capture() || p.board.make_move(mv).is_err() {
            continue;
        }

        let child_params = SearchParams::new(p.board, -p.beta, -p.alpha, p.depth, p.ply);
        let score = -quiescence_search(child_params, thread);
        p.board.take_back();

        if score >= p.beta {
            return p.beta;
        }

        if score > p.alpha {
            p.alpha = score;
        }
    }

    p.alpha
}

/// Returns a statically evaluated `Score` relative to the side being evaluated.
#[inline(always)]
fn evaluate_statically(board: &Board) -> Score {
    // `Negamax` represents the maximizing player, so the score must be relative
    // to the side being evaluated
    let evaluation = evaluation::evaluate(board);
    match board.turn {
        Color::White => evaluation,
        Color::Black => -evaluation,
    }
}
