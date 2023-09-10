macro_rules! impl_binary_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            type Output = $type;

            fn $fn(self, rhs: Self) -> Self::Output {
                $type(std::ops::$trait::$fn(self.0, rhs.0))
            }
        }
    };

    ($type:ident, $generic_type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait<$generic_type> for $type {
            type Output = $type;

            fn $fn(self, rhs: $generic_type) -> Self::Output {
                $type(std::ops::$trait::$fn(self.0, rhs))
            }
        }
    };
}

macro_rules! impl_unary_op {
    ($type:ident, $trait:ident, $fn:ident) => {
        impl std::ops::$trait for $type {
            type Output = $type;

            fn $fn(self) -> Self::Output {
                $type(std::ops::$trait::$fn(self.0))
            }
        }
    };
}

pub(crate) use impl_binary_op;
pub(crate) use impl_unary_op;
