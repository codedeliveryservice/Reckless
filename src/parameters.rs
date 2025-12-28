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
    i32 fp1: 94;
    i32 fp2: 61;
    i32 fp3: 61;
    i32 fp4: 61;
    i32 fp5: 87;
    i32 fp6: 116;
    i32 bnfp1: 68;
    i32 bnfp2: 68;
    i32 bnfp4: 83;
    i32 bnfp5: 24;
    i32 see1: 16;
    i32 see2: 50;
    i32 see3: 21;
    i32 see4: 21;
    i32 see5: 21;
    i32 see6: 25;
    i32 see7: 8;
    i32 see8: 36;
    i32 see9: 33;
    i32 see10: 10;
    i32 lmr1: 1808;
    i32 lmr2: 152;
    i32 lmr3: 152;
    i32 lmr4: 152;
    i32 lmr5: 1564;
    i32 lmr6: 102;
    i32 fds1: 1615;
    i32 fds2: 154;
    i32 fds3: 154;
    i32 fds4: 154;
    i32 fds5: 1444;
    i32 fds6: 65;
);
