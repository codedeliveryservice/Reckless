use super::macros::{impl_assign_op, impl_binary_op, impl_unary_op};

/// Represents a value that determines the odds of winning or losing.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Score(pub i32);

impl Score {
    pub const INVALID: Self = Self(0);

    pub const INFINITY: Self = Self(50000);
    pub const NEGATIVE_INFINITY: Self = Self(-50000);

    pub const CHECKMATE: Self = Self(-48000);
    pub const DRAW: Self = Self(0);

    /// Creates a new `Score`.
    pub fn new(score: i32) -> Self {
        Self(score)
    }
}

impl_unary_op!(Score, Neg, neg);
impl_binary_op!(Score, Add, add);
impl_binary_op!(Score, Sub, sub);
impl_binary_op!(Score, Mul, mul);
impl_binary_op!(Score, Div, div);
impl_assign_op!(Score, AddAssign, add_assign);
impl_assign_op!(Score, SubAssign, sub_assign);

impl std::ops::Add<i32> for Score {
    type Output = Score;

    #[inline(always)]
    fn add(self, rhs: i32) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for Score {
    type Output = Score;

    #[inline(always)]
    fn sub(self, rhs: i32) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
