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
    i32 asp_delta: 10;
    i32 asp_div: 28411;
    i32 delta_base: 43;
    i32 delta_red: 14;
    i32 tthit_scale: 137;
    i32 tthit_sub: 66;
    i32 tthit_cap: 1374;
    i32 tthit_cont_scale: 115;
    i32 tthit_cont_sub: 63;
    i32 tthit_cont_cap: 1478;
    i32 static_scale: 672;
    i32 static_min: 71;
    i32 static_max: 145;
    i32 hs_v1: 2583;
    i32 hs_v2: 868;
    i32 hs_v3: 68;
    i32 razor_offset: 280;
    i32 razor_factor: 255;
    i32 rfp_depth: 7;
    i32 rfp_base: 80;
    i32 rfp_improving: 73;
    i32 rfp_cut: 26;
    i32 rfp_correction: 554;
    i32 rfp_offset: 23;
    i32 nmp_offset: 16;
    i32 nmp_factor: 161;
    i32 nmp_base: 191;
    i32 nmp_eval_factor: 247;
    i32 probcut_base: 289;
    i32 probcut_factor: 61;
    i32 iir_base: 3;
    i32 iir_mul: 3;
    i32 lmr_history_divisor: 7442;
    i32 static_lmp_margin: 19;
    i32 fp_depth: 9;
    i32 fp_base: 125;
    i32 fp_offset: 79;
    i32 bnfp_depth: 6;
    i32 bnfp_base: 111;
    i32 bnfp_move_factor: 396;
    i32 see_quiet_factor: 22;
    i32 see_noisy_factor: 93;
    i32 see_noisy_base: 48;
    i32 see_history_quiet_factor: 41;
    i32 see_history_noisy_factor: 42;
    i32 se_depth_ext_cap: 14;
    i32 se_depth: 5;
    i32 se_double: 17;
    i32 se_triple: 94;
    i32 lmr_v1: 94;
    i32 lmr_v2: 562;
    i32 lmr_v3: 3370;
    i32 lmr_v4: 53;
    i32 lmr_v5: 302;
    i32 lmr_v6: 698;
    i32 lmr_v7: 636;
    i32 lmr_v8: 769;
    i32 lmr_v9: 668;
    i32 lmr_v10: 566;
    i32 lmr_v11: 34;
    i32 lmr_v12: 1182;
    i32 lmr_v13: 838;
    i32 lmr_v14: 869;
    i32 lmr_v15: 736;
    i32 lmr_v16: 58;
    i32 lmr_v17: 7;
    i32 lmr_v18: 1016;
    i32 lmr_v19: 15;
    i32 dod_base: 46;
    i32 dod_factor: 538;
    i32 post_bonus_base: 151;
    i32 post_bonus_offset: 56;
    i32 post_bonus_cap: 1070;
    i32 post_penalty_base: 139;
    i32 post_penalty_offset: 62;
    i32 post_penalty_cap: 1294;
    i32 bonus_noisy_base: 122;
    i32 bonus_noisy_offset: 63;
    i32 bonus_noisy_cap: 1165;
    i32 malus_noisy_base: 149;
    i32 malus_noisy_offset: 76;
    i32 malus_noisy_cap: 1272;
    i32 malus_noisy_move_factor: 14;
    i32 bonus_quiet_base: 146;
    i32 bonus_quiet_offset: 69;
    i32 bonus_quiet_cap: 1464;
    i32 malus_quiet_base: 124;
    i32 malus_quiet_offset: 52;
    i32 malus_quiet_cap: 1377;
    i32 malus_quiet_move_factor: 16;
    i32 bonus_cont_base: 123;
    i32 bonus_cont_offset: 54;
    i32 bonus_cont_cap: 1446;
    i32 malus_cont_base: 228;
    i32 malus_cont_offset: 52;
    i32 malus_cont_cap: 946;
    i32 malus_cont_move_factor: 15;
    i32 pcm_v1: 110;
    i32 pcm_v2: 139;
    i32 pcm_v3: 5;
    i32 pcm_v4: 224;
    i32 pcm_v5: 128;
    i32 pcm_v6: 263;
    i32 pcm_v7: 98;
    i32 pcm_v8: 151;
    i32 pcm_v9: 45;
    i32 pcm_v10: 1579;
    i32 fp_qs: 135;
    i32 corrhist_v1: 1042;
    i32 corrhist_v2: 922;
    i32 corrhist_v3: 802;
    i32 corrhist_v4: 1075;
    i32 corrhist_v5: 939;
    i32 corrhist_v6: 709;
    i32 corrhist_bonus_v1: 982;
    i32 corrhist_bonus_v2: 1097;
    i32 corrhist_bonus_v3: 952;
    i32 corrhist_bonus_v4: 1098;
    i32 corrhist_bonus_v5: 997;
    i32 corrhist_bonus_v6: 1064;
    i32 corrhist_bonus_v7: 970;
    i32 corrhist_clamp_min: 4004;
    i32 corrhist_clamp_max: 3414;
    i32 cont_v1: 1491;
    i32 cont_v2: 1241;
    i32 cont_v3: 964;
    i32 mp_v1: 1110;
    i32 mp_v2: 1068;
    i32 mp_v3: 876;
    i32 mp_v4: 477;
    i32 score_noisy_pc: 2278;
    i32 score_noisy_hist: 952;
    i32 material_base: 22254;
    i32 optimism_base: 1846;
    i32 eval_divisor: 28837;
    i32 optimims_v1: 121;
    i32 optimims_v2: 232;
    i32 alpha_raise_cap: 15;
    i32 corrhist_div: 110;
);
