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

define! {
    i32 asp_delta: 14;
    i32 asp_div: 19919;

    i32 eval_history: 6;
    i32 eval_history_min: 56;
    i32 eval_history_max: 127;

    i32 retro_v1: 4003;
    i32 retro_v2: 609;
    i32 retro_v3: 143;

    i32 razor_v1: 356;
    i32 razor_v2: 331;

    i32 rfp_v1: 6;
    i32 rfp_v2: 61;
    i32 rfp_v3: 75;
    i32 rfp_v4: 29;
    i32 rfp_v5: 574;
    i32 rfp_v6: 12;

    i32 nmp_v1: 18;
    i32 nmp_v2: 143;
    i32 nmp_v3: 227;
    i32 nmp_v4: 203;
    i32 nmp_v5: 3;

    i32 probcut_v1: 269;
    i32 probcut_v2: 31;

    i32 fp_v1: 7;
    i32 fp_v2: 56;
    i32 fp_v3: 94;

    i32 bnp_v1: 9;
    i32 bnp_v2: 132;

    i32 see_v1: 20;
    i32 see_v2: 99;
    i32 see_v3: 49;
    i32 see_v4: 25;

    i32 se_v1: 5;
    i32 se_v2: 32;
    i32 se_v3: 130;

    i32 lmr_v1: 78;
    i32 lmr_v2: 682;
    i32 lmr_v3: 2081;
    i32 lmr_v4: 21;
    i32 lmr_v5: 298;
    i32 lmr_v6: 768;
    i32 lmr_v7: 320;
    i32 lmr_v8: 1074;
    i32 lmr_v9: 530;
    i32 lmr_v10: 851;
    i32 lmr_v11: 1000;
    i32 lmr_v12: 700;
    i32 lmr_v13: 964;
    i32 lmr_v14: 1046;
    i32 lmr_v15: 42;
    i32 lmr_v16: 8;
    i32 lmr_v17: 860;

    i32 dod: 60;

    i32 post_v1: 212;
    i32 post_v2: 88;
    i32 post_v3: 1623;
    i32 post_v4: 60;
    i32 post_v5: 64;
    i32 post_v6: 1320;

    i32 pv_v1: 14;

    i32 bonus_noisy_v1: 115;
    i32 bonus_noisy_v2: 43;
    i32 bonus_noisy_v3: 1195;

    i32 malus_noisy_v1: 154;
    i32 malus_noisy_v2: 45;
    i32 malus_noisy_v3: 1187;
    i32 malus_noisy_v4: 21;

    i32 bonus_quiet_v1: 134;
    i32 bonus_quiet_v2: 82;
    i32 bonus_quiet_v3: 973;

    i32 malus_quiet_v1: 159;
    i32 malus_quiet_v2: 64;
    i32 malus_quiet_v3: 755;
    i32 malus_quiet_v4: 19;

    i32 bonus_cont_v1: 80;
    i32 bonus_cont_v2: 62;
    i32 bonus_cont_v3: 886;

    i32 malus_cont_v1: 146;
    i32 malus_cont_v2: 85;
    i32 malus_cont_v3: 1001;
    i32 malus_cont_v4: 16;

    i32 pcm_v1: 4;
    i32 pcm_v2: 185;
    i32 pcm_v3: 121;
    i32 pcm_v4: 126;
    i32 pcm_v5: 65;
    i32 pcm_v6: 789;

    i32 qs_v1: 128;

    i32 corrhist_v1: 731;
    i32 corrhist_v2: 1028;
    i32 corrhist_v3: 1209;
    i32 corrhist_v4: 803;
    i32 corrhist_v5: 686;

    i32 conthist_v1: 1193;
    i32 conthist_v2: 1016;
    i32 conthist_v3: 1049;

    i32 mp_v1: 38;
    i32 mp_v2: 107;
    i32 mp_v3: 15;
    i32 mp_v4: 1426;
    i32 mp_v5: 823;
    i32 mp_v6: 979;
    i32 mp_v7: 634;
}
