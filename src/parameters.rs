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
    f32 r1: 1000.0;
    f32 r2: 455.0;
    i32 r3: 494;
    i32 r4: 425;
    i32 r5: 1205;
    i32 v1: 106;
    i32 v2: 574;
    i32 v3: 95;
    i32 v4: 557;
    i32 v5: 42;
    i32 v6: 3268;
    i32 v7: 55;
    i32 v8: 303;
    i32 v9: 663;
    i32 v10: 652;
    i32 v11: 783;
    i32 v12: 796;
    i32 v13: 590;
    i32 v14: 573;
    i32 v15: 34;
    i32 v16: 1193;
    i32 v17: 794;
    i32 v18: 1232;
    i32 v19: 768;
    i32 v20: 1024;
);
