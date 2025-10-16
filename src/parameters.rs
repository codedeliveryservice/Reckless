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
    i32 pcm1: 104;
    i32 pcm2: 147;
    i32 pcm3: 217;
    i32 pcm4: 132;
    i32 pcm5: 297;
    i32 pcm6: 100;
    i32 pcm7: 156;
    i32 pcm8: 42;
    i32 pcm9: 1789;
    i32 pcm10: 151;
    i32 pcm11: 41;
    i32 pcm12: 1630;
    i32 pcm13: 6;
);
