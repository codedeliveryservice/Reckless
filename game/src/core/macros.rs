macro_rules! bit_assign_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl $trait for $type {
            #[inline(always)]
            fn $fn(&mut self, rhs: Self) {
                $trait::$fn(&mut self.0, rhs.0);
            }
        }
    };
}

macro_rules! bit_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl $trait for $type {
            type Output = $type;

            #[inline(always)]
            fn $fn(self, rhs: Self) -> Self::Output {
                $type($trait::$fn(self.0, rhs.0))
            }
        }
    };
}

macro_rules! not_op {
    ($type:ident) => {
        impl Not for $type {
            type Output = $type;

            #[inline(always)]
            fn not(self) -> Self::Output {
                Self(!self.0)
            }
        }
    };
}

macro_rules! impl_ops {
    ($type:ident) => {
        use std::ops::*;

        use crate::core::macros::*;

        bit_assign_op!($type, BitXorAssign, bitxor_assign);
        bit_assign_op!($type, BitAndAssign, bitand_assign);
        bit_assign_op!($type, BitOrAssign, bitor_assign);
        bit_assign_op!($type, ShrAssign, shr_assign);
        bit_assign_op!($type, ShlAssign, shl_assign);
        bit_op!($type, BitXor, bitxor);
        bit_op!($type, BitAnd, bitand);
        bit_op!($type, BitOr, bitor);
        bit_op!($type, Shr, shr);
        bit_op!($type, Shl, shl);
        not_op!($type);
    };
}

pub(crate) use bit_assign_op;
pub(crate) use bit_op;
pub(crate) use not_op;

pub(crate) use impl_ops;
