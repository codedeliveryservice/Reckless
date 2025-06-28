use crate::{
    board::Board,
    parameters::*,
    thread::ThreadData,
    types::{PieceType, Score},
};

const MATERIAL_VALUES: [i32; 6] = [132, 414, 432, 661, 1217, 0];
const MATERIAL_BASE: [fn() -> i32; 8] = [v1_base, v2_base, v3_base, v4_base, v5_base, v6_base, v7_base, v8_base];
const MATERIAL_MULT: [fn() -> i32; 8] = [v1_mult, v2_mult, v3_mult, v4_mult, v5_mult, v6_mult, v7_mult, v8_mult];
const MATERIAL_DIV: [fn() -> i32; 8] = [v1_div, v2_div, v3_div, v4_div, v5_div, v6_div, v7_div, v8_div];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let bucket = (td.board.occupancies().len() - 2) / 4;
    let material = material(&td.board);

    eval = (eval * (MATERIAL_BASE[bucket]() + material)
        + td.optimism[td.board.side_to_move()] * (MATERIAL_MULT[bucket]() + material))
        / MATERIAL_DIV[bucket]();

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * MATERIAL_VALUES[pt])
        .sum::<i32>()
}
