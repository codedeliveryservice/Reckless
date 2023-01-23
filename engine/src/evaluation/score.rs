use std::cmp::Ordering;

use game::{impl_bit_assign_op, impl_bit_op};

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct Score(i32);

impl Score {
    pub const EMPTY: Self = Self(0);

    /// Creates a new `Score`.
    pub fn new(score: i32) -> Self {
        Self(score)
    }
}

impl_bit_op!(Score, Add, add);
impl_bit_op!(Score, Sub, sub);
impl_bit_op!(Score, Mul, mul);
impl_bit_op!(Score, Div, div);
impl_bit_assign_op!(Score, AddAssign, add_assign);
impl_bit_assign_op!(Score, SubAssign, sub_assign);

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
