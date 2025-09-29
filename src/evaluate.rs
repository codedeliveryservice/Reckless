use crate::{
    board::Board,
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let material = material(&td.board);
    let nnue_weight = td.nnue.evaluate(&td.board) * (21366 + material);
    let optimism_weight = td.optimism[td.board.side_to_move()] * (1747 + material);

    let mut eval = if nnue_weight == 0 || (nnue_weight.signum() == (nnue_weight + optimism_weight).signum()) {
        nnue_weight + optimism_weight
    } else {
        nnue_weight
    } / 27395;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * PIECE_VALUES[pt])
        .sum::<i32>()
}
