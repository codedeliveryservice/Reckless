pub struct Score;

impl Score {
    pub const DRAW: i32 = 0;
    pub const INVALID: i32 = 0;

    pub const INFINITY: i32 = 32000;

    pub const MATE: i32 = Self::INFINITY - 1000;
    pub const MATE_BOUND: i32 = Self::MATE - 500;
}
