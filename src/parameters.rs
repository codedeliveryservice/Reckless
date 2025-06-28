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
    i32 v1_base: 21682;
    i32 v2_base: 21682;
    i32 v3_base: 21682;
    i32 v4_base: 21682;
    i32 v5_base: 21682;
    i32 v6_base: 21682;
    i32 v7_base: 21682;
    i32 v8_base: 21682;

    i32 v1_mult: 1923;
    i32 v2_mult: 1923;
    i32 v3_mult: 1923;
    i32 v4_mult: 1923;
    i32 v5_mult: 1923;
    i32 v6_mult: 1923;
    i32 v7_mult: 1923;
    i32 v8_mult: 1923;

    i32 v1_div: 28993;
    i32 v2_div: 28993;
    i32 v3_div: 28993;
    i32 v4_div: 28993;
    i32 v5_div: 28993;
    i32 v6_div: 28993;
    i32 v7_div: 28993;
    i32 v8_div: 28993;
);
