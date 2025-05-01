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
    i32 asp_delta: 10;
    i32 asp_div: 32247;

    i32 eval_history: 6;
    i32 eval_history_min: 67;
    i32 eval_history_max: 160;

    i32 retro_v1: 2761;
    i32 retro_v2: 1053;
    i32 retro_v3: 81;

    i32 razor_v1: 307;
    i32 razor_v2: 235;

    i32 rfp_v1: 7;
    i32 rfp_v2: 80;
    i32 rfp_v3: 70;
    i32 rfp_v4: 33;
    i32 rfp_v5: 512;
    i32 rfp_v6: 25;

    i32 nmp_v1: 16;
    i32 nmp_v2: 131;
    i32 nmp_v3: 205;
    i32 nmp_v4: 250;
    i32 nmp_v5: 3;

    i32 probcut_v1: 302;
    i32 probcut_v2: 66;

    i32 fp_v1: 9;
    i32 fp_v2: 93;
    i32 fp_v3: 166;

    i32 bnp_v1: 6;
    i32 bnp_v2: 132;

    i32 see_v1: 19;
    i32 see_v2: 97;
    i32 see_v3: 54;
    i32 see_v4: 37;

    i32 se_v1: 5;
    i32 se_v2: 20;
    i32 se_v3: 132;

    i32 lmr_v1: 84;
    i32 lmr_v2: 554;
    i32 lmr_v3: 4071;
    i32 lmr_v4: 56;
    i32 lmr_v5: 280;
    i32 lmr_v6: 724;
    i32 lmr_v7: 590;
    i32 lmr_v8: 747;
    i32 lmr_v9: 666;
    i32 lmr_v10: 642;
    i32 lmr_v11: 1105;
    i32 lmr_v12: 967;
    i32 lmr_v13: 827;
    i32 lmr_v14: 670;
    i32 lmr_v15: 63;
    i32 lmr_v16: 7;
    i32 lmr_v17: 978;

    i32 dod: 68;

    i32 post_v1: 143;
    i32 post_v2: 65;
    i32 post_v3: 1212;
    i32 post_v4: 135;
    i32 post_v5: 71;
    i32 post_v6: 1177;

    i32 pv_v1: 15;

    i32 bonus_noisy_v1: 133;
    i32 bonus_noisy_v2: 65;
    i32 bonus_noisy_v3: 1270;

    i32 malus_noisy_v1: 143;
    i32 malus_noisy_v2: 75;
    i32 malus_noisy_v3: 1270;
    i32 malus_noisy_v4: 15;

    i32 bonus_quiet_v1: 126;
    i32 bonus_quiet_v2: 75;
    i32 bonus_quiet_v3: 1325;

    i32 malus_quiet_v1: 119;
    i32 malus_quiet_v2: 59;
    i32 malus_quiet_v3: 1180;
    i32 malus_quiet_v4: 18;

    i32 bonus_cont_v1: 139;
    i32 bonus_cont_v2: 61;
    i32 bonus_cont_v3: 1433;

    i32 malus_cont_v1: 171;
    i32 malus_cont_v2: 67;
    i32 malus_cont_v3: 1047;
    i32 malus_cont_v4: 16;

    i32 pcm_v1: 5;
    i32 pcm_v2: 130;
    i32 pcm_v3: 120;
    i32 pcm_v4: 146;
    i32 pcm_v5: 54;
    i32 pcm_v6: 1353;

    i32 qs_v1: 123;

    i32 corrhist_v1: 1181;
    i32 corrhist_v2: 975;
    i32 corrhist_v3: 898;
    i32 corrhist_v4: 972;
    i32 corrhist_v5: 1083;

    i32 conthist_v1: 1164;
    i32 conthist_v2: 1175;
    i32 conthist_v3: 1035;

    i32 mp_v1: 34;
    i32 mp_v2: 107;
    i32 mp_v3: 18;
    i32 mp_v4: 1247;
    i32 mp_v5: 1011;
    i32 mp_v6: 978;
    i32 mp_v7: 517;
}
