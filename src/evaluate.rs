use crate::{
    board::Board,
    parameters::*,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let material = material(&td.board);
    eval = (eval * (20000 + material) + td.optimism[td.board.side_to_move()] * (2000 + material)) / 32768;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    let material_values = [material_pawn(), material_knight(), material_bishop(), material_rook(), material_queen(), 0];

    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * material_values[pt])
        .sum::<i32>()
}
