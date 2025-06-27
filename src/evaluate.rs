use crate::{
    board::Board,
    thread::ThreadData,
    types::{PieceType, Score},
};

const MATERIAL_VALUES: [i32; 6] = [132, 414, 432, 661, 1217, 0];
const MATERIAL_BASE: [i32; 8] = [21480, 21948, 22071, 21384, 20231, 20130, 19557, 21361];
const MATERIAL_MULT: [i32; 8] = [1929, 1970, 1850, 2009, 1937, 2050, 2003, 1801];
const MATERIAL_DIV: [i32; 8] = [29143, 29772, 31584, 31539, 30677, 29954, 30072, 32390];

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
