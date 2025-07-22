pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

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
    i32 basec: 40;
    i32 basel: 40;
    i32 bonus1: 148;
    i32 bonus2: 43;
    i32 bonus3: 1700;
    i32 max_depth: 8;
    i32 v1c: 200;
    i32 v1l: 10;
    i32 v2c: 200;
    i32 v2l: 10;
    i32 v3c: 20;
    i32 v3l: 10;
    i32 v4c: 20;
    i32 v4l: 10;
    i32 param1: 135;
    i32 param2: 102;
);
