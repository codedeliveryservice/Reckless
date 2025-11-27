use crate::{thread::ThreadData, types::Score};

/// Calculates the raw evaluation of the current position from the perspective of the side to move before corrections.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut raw_eval = td.nnue.evaluate(&td.board);

    raw_eval = (raw_eval * (21366 + td.board.material())
        + td.optimism[td.board.side_to_move()] * (1747 + td.board.material()))
        / 27395;

    raw_eval = (raw_eval / 16) * 16 - 1 + (td.board.hash() & 0x2) as i32;

    raw_eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}
