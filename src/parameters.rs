pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

pub const fn lmp_threshold(depth: i32, improving: bool) -> i32 {
    (4 + depth * depth) / (2 - improving as i32)
}

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
            $(println!("option name {} type string default {}", stringify!($name), $value);)*
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

define! {
    i32 asp_delta: 12;
    i32 asp_div: 32292;

    i32 razor_v1: 298;
    i32 razor_v2: 245;

    i32 rfp_v1: 8;
    i32 rfp_v2: 82;
    i32 rfp_v3: 81;
    i32 rfp_v4: 60;

    i32 nmp_v1: 20;
    i32 nmp_v2: 127;
    i32 nmp_v3: 179;
    i32 nmp_v4: 246;

    i32 fp_v1: 10;
    i32 fp_v2: 100;
    i32 fp_v3: 152;

    i32 see_v1: 30;
    i32 see_v2: 100;

    i32 lmr_v1: 523;
    i32 lmr_v2: 1014;
    i32 lmr_v3: 736;
    i32 lmr_v4: 777;
    i32 lmr_v5: 1036;
    i32 lmr_v6: 963;
    i32 lmr_v7: 1032;

    i32 quiet_bonus_v1: 125;
    i32 quiet_bonus_v2: 65;
    i32 quiet_bonus_v3: 1303;

    i32 noisy_bonus_v1: 133;
    i32 noisy_bonus_v2: 64;
    i32 noisy_bonus_v3: 1281;

    i32 cont_bonus_v1: 132;
    i32 cont_bonus_v2: 64;
    i32 cont_bonus_v3: 1210;
}
