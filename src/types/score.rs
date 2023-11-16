use std::ops::{AddAssign, Neg};

pub struct Score;

impl Score {
    pub const DRAW: i32 = 0;
    pub const INVALID: i32 = 0;

    pub const INFINITY: i32 = 32000;
    pub const CHECKMATE: i32 = 31000;
    pub const CHECKMATE_BOUND: i32 = Score::CHECKMATE - 500;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct S(pub i32, pub i32);

impl S {
    pub const fn deconstruct(self) -> (i32, i32) {
        (self.0, self.1)
    }
}

impl AddAssign for S {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl Neg for S {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0, -self.1)
    }
}
