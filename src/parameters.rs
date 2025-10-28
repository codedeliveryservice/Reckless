pub const PIECE_VALUES: [i32; 7] = [109, 403, 435, 679, 1242, 0, 0];

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
    i32 v1: 104;
    i32 v2: 2048;
    i32 v3: 2048;
    i32 v4: 150;
    i32 v5: 147;
    i32 v6: 184;
    i32 v7: 217;
    i32 v8: 132;
    i32 v9: 297;
    i32 v10: 100;
    i32 v11: 156;
    i32 v12: 42;
    i32 v13: 1789;
    i32 v14: 5;
    i32 v15: 8;
    i32 v16: 151;
    i32 v17: 41;
    i32 v18: 1630;
);
