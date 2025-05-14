use crate::{
    board::Board, parameters::*,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    eval = eval * (material_base() + material(&td.board)) / 32768;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    let material_values = [material_pawn(), material_knight(), material_bishop(), material_rook(), material_queen(), 0];

    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * material_values[pt])
        .sum::<i32>()
}
