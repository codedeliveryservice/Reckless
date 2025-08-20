use crate::{
    board::Board,
    thread::ThreadData,
    types::{PieceType, Score},
};

const MATERIAL_VALUES: [i32; 6] = [129, 403, 435, 679, 1242, 0];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let material = material(&td.board);

    eval = (eval * (21063 + material) + td.optimism[td.board.side_to_move()] * (1817 + material)) / 27627;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * MATERIAL_VALUES[pt])
        .sum::<i32>()
}
