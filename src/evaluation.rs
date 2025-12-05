use crate::{thread::ThreadData, types::Score};

pub fn correct_eval(td: &ThreadData, raw_eval: i32, correction_value: i32) -> i32 {
    let material = td.board.material();
    let optimism = td.optimism[td.board.side_to_move()];

    let mut eval = (raw_eval * (21366 + material) + optimism * material) / 27395;

    eval = (eval / 16) * 16;

    eval = eval * (200 - td.board.halfmove_clock() as i32) / 200;

    eval += correction_value;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}
