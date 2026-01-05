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
    i32 lmr1: 260;
    i32 lmr2: 68;
    i32 lmr3: 2031;
    i32 lmr4: 1563;
    i32 lmr5: 120;
    i32 lmr6: 1024;
    i32 lmr7: 100;
    i32 lmr8: 1024;
    i32 lmr9: 146;
    i32 fds1: 246;
    i32 fds2: 55;
    i32 fds3: 1634;
    i32 fds4: 1423;
    i32 fds5: 120;
    i32 fds6: 1024;
    i32 fds7: 100;
    i32 fds8: 1024;
);
