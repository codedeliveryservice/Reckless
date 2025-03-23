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
    i32 asp_div: 33385;

    i32 razor_v1: 306;
    i32 razor_v2: 249;

    i32 rfp_v1: 8;
    i32 rfp_v2: 80;
    i32 rfp_v3: 82;
    i32 rfp_v4: 61;

    i32 nmp_v1: 20;
    i32 nmp_v2: 129;
    i32 nmp_v3: 184;
    i32 nmp_v4: 251;

    i32 probcut_v1: 247;
    i32 probcut_v2: 66;

    i32 fp_v1: 10;
    i32 fp_v2: 100;
    i32 fp_v3: 147;

    i32 see_v1: 30;
    i32 see_v2: 95;

    i32 se_v1: 24;
    i32 se_v2: 135;

    i32 lmr_v1: 512;
    i32 lmr_v2: 16584;
    i32 lmr_v3: 1016;
    i32 lmr_v4: 794;
    i32 lmr_v5: 758;
    i32 lmr_v6: 1054;
    i32 lmr_v7: 999;
    i32 lmr_v8: 989;

    i32 dod_v1: 64;
    i32 qs_v1: 126;

    i32 corr_v1: 127;
    i32 corr_v2: 129;
    i32 corr_v3: 129;
    i32 corr_v4: 128;
    i32 corr_v5: 127;
    i32 corr_v6: 127;

    i32 quiet_bonus_v1: 129;
    i32 quiet_bonus_v2: 64;
    i32 quiet_bonus_v3: 1321;

    i32 quiet_malus_v1: 127;
    i32 quiet_malus_v2: 63;
    i32 quiet_malus_v3: 1286;

    i32 noisy_bonus_v1: 129;
    i32 noisy_bonus_v2: 63;
    i32 noisy_bonus_v3: 1294;

    i32 noisy_malus_v1: 128;
    i32 noisy_malus_v2: 66;
    i32 noisy_malus_v3: 1296;

    i32 cont1_bonus_v1: 130;
    i32 cont1_bonus_v2: 65;
    i32 cont1_bonus_v3: 1297;

    i32 cont1_malus_v1: 134;
    i32 cont1_malus_v2: 63;
    i32 cont1_malus_v3: 1244;

    i32 cont2_bonus_v1: 124;
    i32 cont2_bonus_v2: 63;
    i32 cont2_bonus_v3: 1266;

    i32 cont2_malus_v1: 127;
    i32 cont2_malus_v2: 64;
    i32 cont2_malus_v3: 1261;
}
