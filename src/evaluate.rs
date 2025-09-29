use crate::{
    board::Board,
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{PieceType, Score},
};

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let (our_material, total_material) = material(&td.board);

    eval = (eval * (21366 + total_material) + td.optimism[td.board.side_to_move()] * (1747 + 2 * our_material)) / 27395;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> (i32, i32) {
    let mut our = 0;
    let mut total = 0;

    for &pt in &[PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen] {
        let count_all = board.pieces(pt).len() as i32;
        let count_our = board.our(pt).len() as i32;

        total += count_all * PIECE_VALUES[pt];
        our += count_our * PIECE_VALUES[pt];
    }

    (our, total)
}
