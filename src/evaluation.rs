use crate::{thread::ThreadData, types::Score};

pub fn correct_eval(_: &ThreadData, raw_eval: i32, correction_value: i32) -> i32 {
    let mut eval = raw_eval;

    eval += correction_value;

    eval.clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}
