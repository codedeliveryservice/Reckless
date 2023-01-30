use std::cmp::Ordering;

use game::{impl_assign_op, impl_binary_op, impl_unary_op};

/// Represents a value that determines the odds of winning or losing.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Score(pub i32);

impl Score {
    pub const ZERO: Self = Self(0);

    /// Creates a new `Score`.
    pub fn new(score: i32) -> Self {
        Self(score)
    }

    /// Returns a `Score` shifted by the specified offset.
    pub fn shift(self, offset: i32) -> Self {
        Self(self.0 + offset)
    }
}

impl_unary_op!(Score, Neg, neg);
impl_binary_op!(Score, Add, add);
impl_binary_op!(Score, Sub, sub);
impl_binary_op!(Score, Mul, mul);
impl_binary_op!(Score, Div, div);
impl_assign_op!(Score, AddAssign, add_assign);
impl_assign_op!(Score, SubAssign, sub_assign);

impl std::fmt::Display for Score {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0.cmp(&0) {
            Ordering::Equal => write!(f, "=")?,
            Ordering::Greater => write!(f, "+")?,
            Ordering::Less => { /* Negative sign included in the number */ }
        };

        write!(f, "{:.2}", self.0 as f32 / 100f32)
    }
}
