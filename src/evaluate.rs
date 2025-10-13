use crate::{
    board::Board,
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let material = 320 * td.board.pieces(PieceType::Pawn).len() as i32 + non_pawn_material(&td.board);

    eval = (eval * (21366 + material) + td.optimism[td.board.side_to_move()] * (1747 + material)) / 27395;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn non_pawn_material(board: &Board) -> i32 {
    use PieceType::*;
    [Knight, Bishop, Rook, Queen].iter().map(|&pt| board.pieces(pt).len() as i32 * PIECE_VALUES[pt]).sum()
}
