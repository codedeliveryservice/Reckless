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
    i32 asp_delta: 11;
    i32 asp_div: 29619;

    i32 eval_history: 6;
    i32 eval_history_min: 68;
    i32 eval_history_max: 137;

    i32 retro_v1: 2890;
    i32 retro_v2: 904;
    i32 retro_v3: 90;

    i32 razor_v1: 312;
    i32 razor_v2: 254;

    i32 rfp_v1: 8;
    i32 rfp_v2: 78;
    i32 rfp_v3: 77;
    i32 rfp_v4: 28;
    i32 rfp_v5: 508;
    i32 rfp_v6: 25;

    i32 nmp_v1: 18;
    i32 nmp_v2: 126;
    i32 nmp_v3: 192;
    i32 nmp_v4: 267;
    i32 nmp_v5: 3;

    i32 probcut_v1: 279;
    i32 probcut_v2: 60;

    i32 fp_v1: 9;
    i32 fp_v2: 96;
    i32 fp_v3: 160;

    i32 bnp_v1: 6;
    i32 bnp_v2: 128;

    i32 see_v1: 27;
    i32 see_v2: 94;
    i32 see_v3: 52;
    i32 see_v4: 31;

    i32 se_v1: 6;
    i32 se_v2: 24;
    i32 se_v3: 126;

    i32 lmr_v1: 77;
    i32 lmr_v2: 525;
    i32 lmr_v3: 4081;
    i32 lmr_v4: 64;
    i32 lmr_v5: 282;
    i32 lmr_v6: 716;
    i32 lmr_v7: 657;
    i32 lmr_v8: 761;
    i32 lmr_v9: 724;
    i32 lmr_v10: 738;
    i32 lmr_v11: 1105;
    i32 lmr_v12: 1000;
    i32 lmr_v13: 1040;
    i32 lmr_v14: 813;
    i32 lmr_v15: 69;
    i32 lmr_v16: 8;
    i32 lmr_v17: 973;

    i32 dod: 70;

    i32 post_v1: 142;
    i32 post_v2: 64;
    i32 post_v3: 1204;
    i32 post_v4: 127;
    i32 post_v5: 68;
    i32 post_v6: 1326;

    i32 pv_v1: 17;

    i32 bonus_noisy_v1: 130;
    i32 bonus_noisy_v2: 62;
    i32 bonus_noisy_v3: 1288;

    i32 malus_noisy_v1: 127;
    i32 malus_noisy_v2: 64;
    i32 malus_noisy_v3: 1315;
    i32 malus_noisy_v4: 15;

    i32 bonus_quiet_v1: 123;
    i32 bonus_quiet_v2: 63;
    i32 bonus_quiet_v3: 1301;

    i32 malus_quiet_v1: 123;
    i32 malus_quiet_v2: 64;
    i32 malus_quiet_v3: 1235;
    i32 malus_quiet_v4: 16;

    i32 bonus_cont_v1: 133;
    i32 bonus_cont_v2: 64;
    i32 bonus_cont_v3: 1300;

    i32 malus_cont_v1: 143;
    i32 malus_cont_v2: 65;
    i32 malus_cont_v3: 1312;
    i32 malus_cont_v4: 15;

    i32 pcm_v1: 5;
    i32 pcm_v2: 124;
    i32 pcm_v3: 125;
    i32 pcm_v4: 131;
    i32 pcm_v5: 55;
    i32 pcm_v6: 1373;

    i32 qs_v1: 131;

    i32 corrhist_v1: 1050;
    i32 corrhist_v2: 983;
    i32 corrhist_v3: 952;
    i32 corrhist_v4: 968;
    i32 corrhist_v5: 1007;

    i32 conthist_v1: 1055;
    i32 conthist_v2: 1126;
    i32 conthist_v3: 1043;

    i32 mp_v1: 33;
    i32 mp_v2: 103;
    i32 mp_v3: 17;
    i32 mp_v4: 1153;
    i32 mp_v5: 1090;
    i32 mp_v6: 1051;
    i32 mp_v7: 529;
}
