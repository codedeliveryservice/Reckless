use crate::{board::Board, parameters::PIECE_VALUES, thread::ThreadData, types::PieceType};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    #[cfg(not(feature = "datagen"))]
    {
        eval = eval * (700 + count_material(&td.board) / 32) / 1024;
    }

    eval.clamp(-16384, 16384)
}

fn count_material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * PIECE_VALUES[pt])
        .sum::<i32>()
}
