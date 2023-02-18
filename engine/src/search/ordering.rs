use game::{Move, MoveList};

use super::{mvv_lva, SearchParams, SearchThread};

pub fn order_moves(p: &SearchParams, thread: &SearchThread) -> MoveList {
    let mut moves = p.board.generate_moves();

    let mut scores = vec![0; moves.len()];
    for index in 0..moves.len() {
        scores[index] = score_move(moves[index], p, thread);
    }

    for current in 0..moves.len() {
        for compared in (current + 1)..moves.len() {
            if scores[current] < scores[compared] {
                scores.swap(current, compared);
                moves.swap(current, compared);
            }
        }
    }

    moves
}

/// Returns a move score based on heuristic analysis.
fn score_move(mv: Move, p: &SearchParams, thread: &SearchThread) -> u32 {
    if mv.is_capture() {
        return mvv_lva::score_mvv_lva(p.board, mv);
    }

    if thread.killers.contains(mv, p.ply) {
        // The quiet move score is rated below any capture move
        return 90;
    }

    Default::default()
}
