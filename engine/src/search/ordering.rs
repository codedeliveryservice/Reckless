use game::{Board, Move, MoveList};

use super::{killer_moves::KillerMoves, mvv_lva};

pub fn order_moves(board: &mut Board, killers: &KillerMoves<2>, ply: usize) -> MoveList {
    let mut moves = board.generate_moves();

    let mut scores = vec![0; moves.len()];
    for index in 0..moves.len() {
        scores[index] = score_move(board, moves[index], killers, ply);
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
fn score_move(board: &Board, mv: Move, killers: &KillerMoves<2>, ply: usize) -> u32 {
    if mv.is_capture() {
        return mvv_lva::score_mvv_lva(board, mv);
    }

    if killers.contains(mv, ply) {
        // The quiet move score is rated below any capture move
        return 90;
    }

    Default::default()
}
