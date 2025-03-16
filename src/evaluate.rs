use crate::{board::Board, thread::ThreadData, types::PieceType};

const MAX_PHASE: i32 = 62;
const PHASE_WEIGHTS: [i32; PieceType::NUM - 1] = [0, 3, 3, 5, 9];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    #[cfg(not(feature = "datagen"))]
    {
        eval -= eval * (MAX_PHASE - game_phase(&td.board)) / (5 * MAX_PHASE);
    }

    eval.clamp(-16384, 16384)
}

fn game_phase(board: &Board) -> i32 {
    [PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&piece| board.pieces(piece).len() as i32 * PHASE_WEIGHTS[piece])
        .sum::<i32>()
        .min(MAX_PHASE)
}
