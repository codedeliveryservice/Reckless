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
    i32 lmr1: 489;
    i32 lmr2: 412;
    i32 lmr3: 1243;
    i32 lmr4: 489;
    i32 lmr5: 137;
    i32 lmr6: 488;
    i32 lmr7: 109;
    i32 lmr8: 46;
    i32 lmr9: 3607;
    i32 lmr10: 69;
    i32 lmr11: 427;
    i32 lmr12: 677;
    i32 lmr13: 729;
    i32 lmr14: 393;
    i32 lmr15: 552;
    i32 lmr16: 1675;
    i32 lmr17: 934;
    i32 lmr18: 1049;
    i32 lmr19: 1555;
    i32 lmr20: 791;
    i32 lmr21: 1397;
    i32 dod1: 37;
    i32 dod2: 495;
    i32 post1: 155;
    i32 post2: 63;
    i32 post3: 851;
    i32 fds1: 380;
    i32 fds2: 153;
    i32 fds3: 355;
    i32 fds4: 68;
    i32 fds5: 47;
    i32 fds6: 2667;
    i32 fds7: 52;
    i32 fds8: 750;
    i32 fds9: 537;
    i32 fds10: 1081;
    i32 fds11: 491;
    i32 fds12: 780;
    i32 fds13: 1478;
    i32 fds14: 1048;
    i32 fds15: 744;
    i32 fds16: 1438;
    i32 fds17: 849;
    i32 fds18: 1052;
    i32 fds19: 3034;
);
