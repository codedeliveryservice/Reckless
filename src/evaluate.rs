use crate::{
    board::Board,
    thread::ThreadData,
    types::{PieceType, Score},
};

const MATERIAL_VALUES: [i32; 6] = [132, 414, 432, 661, 1217, 0];
const MATERIAL_BASE: [i32; 8] = [22324, 22185, 20670, 20807, 20825, 22232, 23045, 22593];
const MATERIAL_MULT: [i32; 8] = [1952, 1896, 2003, 1814, 1878, 1964, 1892, 1980];
const MATERIAL_DIV: [i32; 8] = [28529, 30004, 29633, 29126, 28955, 28400, 29051, 28837];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    let bucket = (td.board.occupancies().len() - 2) / 4;
    let material = material(&td.board);

    eval = (eval * (MATERIAL_BASE[bucket] + material)
        + td.optimism[td.board.side_to_move()] * (MATERIAL_MULT[bucket] + material))
        / MATERIAL_DIV[bucket];

    eval = (eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * MATERIAL_VALUES[pt])
        .sum::<i32>()
}
