use std::ops::{AddAssign, Neg};

#[derive(Debug, Default, Clone, Copy)]
pub struct S(pub i32, pub i32);

impl S {
    pub fn deconstruct(&self) -> (i32, i32) {
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
        S(-self.0, -self.1)
    }
}
