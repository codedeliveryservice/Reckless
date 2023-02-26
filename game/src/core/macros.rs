macro_rules! impl_assign_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            #[inline(always)]
            fn $fn(&mut self, rhs: Self) {
                std::ops::$trait::$fn(&mut self.0, rhs.0);
            }
        }
    };
}

macro_rules! impl_binary_op {
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

macro_rules! impl_unary_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            type Output = $type;

            #[inline(always)]
            fn $fn(self) -> Self::Output {
                $type(std::ops::$trait::$fn(self.0))
            }
        }
    };
}

pub(crate) use impl_assign_op;
pub(crate) use impl_binary_op;
pub(crate) use impl_unary_op;
