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
    i32 lmr1: 481;
    i32 lmr2: 419;
    i32 lmr3: 1269;
    i32 lmr4: 488;
    i32 lmr5: 128;
    i32 lmr6: 493;
    i32 lmr7: 112;
    i32 lmr8: 43;
    i32 lmr9: 3175;
    i32 lmr10: 67;
    i32 lmr11: 445;
    i32 lmr12: 733;
    i32 lmr13: 702;
    i32 lmr14: 407;
    i32 lmr15: 577;
    i32 lmr16: 1798;
    i32 lmr17: 938;
    i32 lmr18: 1129;
    i32 lmr19: 1511;
    i32 lmr20: 751;
    i32 lmr21: 1520;
    i32 dod1: 36;
    i32 dod2: 482;
    i32 post1: 147;
    i32 post2: 60;
    i32 post3: 832;
    i32 fds1: 385;
    i32 fds2: 148;
    i32 fds3: 360;
    i32 fds4: 66;
    i32 fds5: 46;
    i32 fds6: 2712;
    i32 fds7: 54;
    i32 fds8: 784;
    i32 fds9: 554;
    i32 fds10: 1075;
    i32 fds11: 475;
    i32 fds12: 780;
    i32 fds13: 1582;
    i32 fds14: 1036;
    i32 fds15: 715;
    i32 fds16: 1481;
    i32 fds17: 817;
    i32 fds18: 1020;
    i32 fds19: 3060;
);
