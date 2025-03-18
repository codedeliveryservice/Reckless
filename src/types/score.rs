use super::MAX_PLY;

pub struct Score;

#[rustfmt::skip]
impl Score {
    pub const ZERO: i32 = 0;
    pub const DRAW: i32 = 0;

    pub const NONE:     i32 = 32002;
    pub const INFINITE: i32 = 32001;
    pub const MATE:     i32 = 32000;

    pub const MATE_IN_MAX:  i32 =  32000 - MAX_PLY as i32;
    pub const MATED_IN_MAX: i32 = -32000 + MAX_PLY as i32;
}

pub const fn mated_in(ply: usize) -> i32 {
    -Score::MATE + ply as i32
}

pub const fn mate_in(ply: usize) -> i32 {
    Score::MATE - ply as i32
}

pub const fn is_win(score: i32) -> bool {
    score >= Score::MATE_IN_MAX
}

pub const fn is_loss(score: i32) -> bool {
    score <= Score::MATED_IN_MAX
}

pub const fn is_decisive(score: i32) -> bool {
    is_win(score) || is_loss(score)
}
