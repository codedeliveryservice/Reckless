use crate::{
    board::Board,
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{PieceType, Score},
};

pub fn evaluate(td: &ThreadData, mut eval: i32, correction_value: i32) -> i32 {
    let material = material(&td.board);

    eval = (eval * (21366 + material) + td.optimism[td.board.side_to_move()] * (1747 + material)) / 27395;

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval = (eval * (200 - td.board.halfmove_clock() as i32)) / 200;

    eval += correction_value;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

pub fn evaluate_qs(td: &ThreadData, mut eval: i32, correction_value: i32) -> i32 {
    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval = (eval * (200 - td.board.halfmove_clock() as i32)) / 200;

    eval += correction_value;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * PIECE_VALUES[pt])
        .sum::<i32>()
}
