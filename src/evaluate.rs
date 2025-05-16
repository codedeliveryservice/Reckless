use crate::{
    board::Board,
    thread::ThreadData,
    types::{PieceType, Score},
};

const MATERIAL_VALUES: [i32; 6] = [132, 414, 432, 661, 1217, 0];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    eval = eval * (20099 + material(&td.board)) / 32768;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * MATERIAL_VALUES[pt])
        .sum::<i32>()
}
