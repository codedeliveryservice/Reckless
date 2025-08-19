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

    eval = (eval * (eval1() + material) + td.optimism[td.board.side_to_move()] * (eval2() + material)) / eval3();

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    let material_values = [mat1(), mat2(), mat3(), mat4(), mat5(), 0];

    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * material_values[pt])
        .sum::<i32>()
}
