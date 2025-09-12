use crate::types::S;

pub const PIECE_VALUES: [fn() -> S; 7] = [
    || S(pawn_mg(), pawn_eg()),
    || S(knight_mg(), knight_eg()),
    || S(bishop_mg(), bishop_eg()),
    || S(rook_mg(), rook_eg()),
    || S(queen_mg(), queen_eg()),
    || S(0, 0),
    || S(0, 0),
];

#[allow(unused_macros)]
#[cfg(not(feature = "spsa"))]
macro_rules! define {
    {$($type:ident $name:ident: $value:expr; )*} => {
        $(pub const fn $name() -> $type {
            $value
        })*
    };
}

#[cfg(feature = "spsa")]
macro_rules! define {
    {$($type:ident $name:ident: $value:expr; )*} => {
        pub fn set_parameter(name: &str, value: &str) {
            match name {
                $(stringify!($name) => unsafe { parameters::$name = value.parse().unwrap() },)*
                _ => panic!("Unknown tunable parameter: {name}"),
            }
        }

        pub fn print_options() {
            $(println!("option name {} type string", stringify!($name));)*
        }

        $(pub fn $name() -> $type {
            unsafe { parameters::$name }
        })*

        #[allow(non_upper_case_globals)]
        mod parameters {
            $(pub static mut $name: $type = $value;)*
        }
    };
}

define!(
    i32 pawn_mg: 100;
    i32 pawn_eg: 100;
    i32 knight_mg: 375;
    i32 knight_eg: 375;
    i32 bishop_mg: 400;
    i32 bishop_eg: 400;
    i32 rook_mg: 625;
    i32 rook_eg: 625;
    i32 queen_mg: 1200;
    i32 queen_eg: 1200;
);
