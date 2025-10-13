use crate::{
    board::Board,
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let material = material(&td.board);

    eval = (eval * (21366 + material) + td.optimism[td.board.side_to_move()] * (1747 + material)) / 27395;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    use PieceType::*;
    [Pawn, Knight, Bishop, Rook, Queen]
        .iter()
        .map(|&pt| {
            let value = if pt == Pawn { 500 } else { PIECE_VALUES[pt] };
            board.pieces(pt).len() as i32 * value
        })
        .sum::<i32>()
}
