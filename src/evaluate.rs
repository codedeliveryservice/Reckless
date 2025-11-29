use crate::thread::ThreadData;

pub fn evaluate(td: &mut ThreadData) -> i32 {
    td.nnue.evaluate(&td.board)
}
