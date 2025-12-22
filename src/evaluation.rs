use crate::{thread::ThreadData, types::Score};

pub fn correct_eval(td: &ThreadData, raw_eval: i32, correction_value: i32) -> i32 {
    let mut eval = (raw_eval * (21372 + td.board.material())
        + td.optimism[td.board.side_to_move()] * (1536 + td.board.material()))
        / 27380;

    eval = eval * (200 - td.board.halfmove_clock() as i32) / 200;

    eval += correction_value;

    eval = (eval / 16) * 16;

    eval += (td.board.hash() & 0x2) as i32 - 1;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}
