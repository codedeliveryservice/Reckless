pub const PIECE_VALUES: [fn() -> i32; 7] = [pawn, knight, bishop, rook, queen, || 0, || 0];

pub const fn lmp_threshold(depth: i32, improving: bool) -> i32 {
    (4 + depth * depth) / (2 - improving as i32)
}

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

define! {
    i32 ab_threshold_v1: 34;
    i32 ab_threshold_v2: 107;
    i32 qs_threshold_v1: 34;
    i32 qs_threshold_v2: 107;
    i32 pawn: 100;
    i32 knight: 375;
    i32 bishop: 400;
    i32 rook: 625;
    i32 queen: 1200;
    i32 pruning_quite_v1: 21;
    i32 pruning_quite_v2: 42;
    i32 pruning_quite_v3: 0;
    i32 pruning_noisy_v1: 98;
    i32 pruning_noisy_v2: 42;
    i32 pruning_noisy_v3: 50;
}
