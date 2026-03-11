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
    i32 v1: 238;
    i32 v2: 57;
    i32 v3: 2513;
    i32 v4: 1427;
    i32 v5: 158;
    i32 v6: 1098;
    i32 v7: 64;
    i32 v8: 897;
    i32 v9: 1127;
    i32 v10: 1450;
    i32 v11: 2200;
    i32 v12: 454;
    i32 v13: 254;
    i32 v14: 1368;
    i32 v15: 1452;
    i32 v16: 3316;
    i32 v17: 512;
    i32 v18: 128;
    i32 v19: 3072;
);
