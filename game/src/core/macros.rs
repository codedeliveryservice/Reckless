#[macro_export]
macro_rules! impl_bit_assign_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            #[inline(always)]
            fn $fn(&mut self, rhs: Self) {
                std::ops::$trait::$fn(&mut self.0, rhs.0);
            }
        }
    };
}

#[macro_export]
macro_rules! impl_bit_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            type Output = $type;

            #[inline(always)]
            fn $fn(self, rhs: Self) -> Self::Output {
                $type(std::ops::$trait::$fn(self.0, rhs.0))
            }
        }
    };
}

#[macro_export]
macro_rules! impl_not_op {
    ($type:ident) => {
        impl std::ops::Not for $type {
            type Output = $type;

            #[inline(always)]
            fn not(self) -> Self::Output {
                Self(!self.0)
            }
        }
    };
}

pub(crate) use impl_bit_assign_op;
pub(crate) use impl_bit_op;
pub(crate) use impl_not_op;
