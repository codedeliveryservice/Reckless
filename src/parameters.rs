pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

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
    i32 asp_div: 27269;
    i32 delta_base: 44;
    i32 tthit_scale: 138;
    i32 tthit_sub: 71;
    i32 tthit_cap: 1419;
    i32 tthit_cont_scale: 109;
    i32 tthit_cont_sub: 64;
    i32 tthit_cont_cap: 1381;
    i32 static_scale: 668;
    i32 static_min: 64;
    i32 static_max: 149;
    i32 hs_v1: 2596;
    i32 hs_v2: 922;
    i32 hs_v3: 72;
    i32 razor_offset: 295;
    i32 razor_factor: 266;
    i32 rfp_base: 79;
    i32 rfp_improving: 69;
    i32 rfp_cut: 26;
    i32 rfp_correction: 540;
    i32 rfp_offset: 25;
    i32 nmp_offset: 16;
    i32 nmp_factor: 155;
    i32 nmp_base: 195;
    i32 nmp_eval_factor: 231;
    i32 probcut_base: 273;
    i32 probcut_factor: 62;
    i32 lmr_history_divisor: 7452;
    i32 static_lmp_margin: 16;
    i32 fp_base: 125;
    i32 fp_offset: 76;
    i32 bnfp_base: 113;
    i32 bnfp_move_factor: 398;
    i32 see_quiet_factor: 24;
    i32 see_noisy_factor: 95;
    i32 see_noisy_base: 46;
    i32 see_history_quiet_factor: 43;
    i32 se_depth_ext_cap: 14;
    i32 se_triple: 101;
    i32 lmr_v1: 98;
    i32 lmr_v2: 580;
    i32 lmr_v3: 3258;
    i32 lmr_v4: 54;
    i32 lmr_v5: 295;
    i32 lmr_v6: 679;
    i32 lmr_v7: 657;
    i32 lmr_v8: 784;
    i32 lmr_v9: 641;
    i32 lmr_v10: 566;
    i32 lmr_v11: 35;
    i32 lmr_v12: 1157;
    i32 lmr_v13: 807;
    i32 lmr_v14: 824;
    i32 lmr_v15: 743;
    i32 lmr_v16: 56;
    i32 lmr_v18: 960;
    i32 dod_factor: 515;
    i32 post_bonus_base: 153;
    i32 post_bonus_offset: 52;
    i32 post_bonus_cap: 954;
    i32 post_penalty_base: 137;
    i32 post_penalty_offset: 62;
    i32 post_penalty_cap: 1208;
    i32 bonus_noisy_base: 123;
    i32 bonus_noisy_offset: 64;
    i32 bonus_noisy_cap: 1172;
    i32 malus_noisy_base: 146;
    i32 malus_noisy_offset: 75;
    i32 malus_noisy_cap: 1349;
    i32 bonus_quiet_base: 148;
    i32 bonus_quiet_offset: 70;
    i32 bonus_quiet_cap: 1479;
    i32 malus_quiet_base: 129;
    i32 malus_quiet_offset: 54;
    i32 malus_quiet_cap: 1247;
    i32 malus_quiet_move_factor: 17;
    i32 bonus_cont_base: 118;
    i32 bonus_cont_offset: 52;
    i32 bonus_cont_cap: 1357;
    i32 malus_cont_base: 243;
    i32 malus_cont_offset: 51;
    i32 malus_cont_cap: 918;
    i32 pcm_v1: 104;
    i32 pcm_v2: 141;
    i32 pcm_v4: 224;
    i32 pcm_v5: 130;
    i32 pcm_v6: 279;
    i32 pcm_v7: 102;
    i32 pcm_v8: 140;
    i32 pcm_v9: 43;
    i32 pcm_v10: 1581;
    i32 fp_qs: 135;
    i32 corrhist_v1: 1071;
    i32 corrhist_v2: 899;
    i32 corrhist_v3: 732;
    i32 corrhist_v4: 1086;
    i32 corrhist_v5: 952;
    i32 corrhist_v6: 684;
    i32 corrhist_bonus_v1: 1007;
    i32 corrhist_bonus_v2: 1141;
    i32 corrhist_bonus_v3: 945;
    i32 corrhist_bonus_v4: 1120;
    i32 corrhist_bonus_v5: 1037;
    i32 corrhist_bonus_v6: 1019;
    i32 corrhist_bonus_v7: 988;
    i32 corrhist_clamp_min: 3834;
    i32 corrhist_clamp_max: 3329;
    i32 cont_v1: 1530;
    i32 cont_v2: 1170;
    i32 cont_v3: 941;
    i32 mp_v1: 1185;
    i32 mp_v2: 1037;
    i32 mp_v3: 880;
    i32 mp_v4: 466;
    i32 score_noisy_pc: 2155;
    i32 score_noisy_hist: 935;
    i32 material_base: 22550;
    i32 optimism_base: 1876;
    i32 eval_divisor: 29772;
    i32 optimims_v1: 116;
    i32 optimims_v2: 238;
    i32 alpha_raise_cap: 15;
    i32 corrhist_div: 111;
);
