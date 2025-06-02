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
    i32 asp_div: 26411;
    i32 delta_base: 44;
    i32 tthit_scale: 137;
    i32 tthit_sub: 73;
    i32 tthit_cap: 1405;
    i32 tthit_cont_scale: 105;
    i32 tthit_cont_sub: 63;
    i32 tthit_cont_cap: 1435;
    i32 static_scale: 674;
    i32 static_min: 61;
    i32 static_max: 144;
    i32 hs_v1: 2691;
    i32 hs_v2: 905;
    i32 hs_v3: 69;
    i32 razor_offset: 303;
    i32 razor_factor: 260;
    i32 rfp_base: 80;
    i32 rfp_improving: 72;
    i32 rfp_cut: 25;
    i32 rfp_correction: 556;
    i32 rfp_offset: 24;
    i32 nmp_offset: 15;
    i32 nmp_factor: 159;
    i32 nmp_base: 203;
    i32 nmp_eval_factor: 225;
    i32 probcut_base: 280;
    i32 probcut_factor: 63;
    i32 lmr_history_divisor: 7657;
    i32 static_lmp_margin: 18;
    i32 fp_base: 122;
    i32 fp_offset: 78;
    i32 bnfp_base: 111;
    i32 bnfp_move_factor: 396;
    i32 see_quiet_factor: 24;
    i32 see_noisy_factor: 94;
    i32 see_noisy_base: 48;
    i32 see_history_quiet_factor: 43;
    i32 se_depth_ext_cap: 14;
    i32 se_triple: 101;
    i32 lmr_v1: 98;
    i32 lmr_v2: 568;
    i32 lmr_v3: 3295;
    i32 lmr_v4: 54;
    i32 lmr_v5: 295;
    i32 lmr_v6: 683;
    i32 lmr_v7: 647;
    i32 lmr_v8: 791;
    i32 lmr_v9: 614;
    i32 lmr_v10: 576;
    i32 lmr_v11: 34;
    i32 lmr_v12: 1141;
    i32 lmr_v13: 820;
    i32 lmr_v14: 800;
    i32 lmr_v15: 732;
    i32 lmr_v16: 55;
    i32 lmr_v18: 955;
    i32 dod_factor: 512;
    i32 post_bonus_base: 152;
    i32 post_bonus_offset: 50;
    i32 post_bonus_cap: 973;
    i32 post_penalty_base: 139;
    i32 post_penalty_offset: 63;
    i32 post_penalty_cap: 1166;
    i32 bonus_noisy_base: 124;
    i32 bonus_noisy_offset: 65;
    i32 bonus_noisy_cap: 1177;
    i32 malus_noisy_base: 145;
    i32 malus_noisy_offset: 75;
    i32 malus_noisy_cap: 1403;
    i32 bonus_quiet_base: 148;
    i32 bonus_quiet_offset: 71;
    i32 bonus_quiet_cap: 1458;
    i32 malus_quiet_base: 125;
    i32 malus_quiet_offset: 52;
    i32 malus_quiet_cap: 1263;
    i32 malus_quiet_move_factor: 17;
    i32 bonus_cont_base: 114;
    i32 bonus_cont_offset: 53;
    i32 bonus_cont_cap: 1318;
    i32 malus_cont_base: 244;
    i32 malus_cont_offset: 51;
    i32 malus_cont_cap: 907;
    i32 pcm_v1: 102;
    i32 pcm_v2: 141;
    i32 pcm_v4: 227;
    i32 pcm_v5: 129;
    i32 pcm_v6: 277;
    i32 pcm_v7: 101;
    i32 pcm_v8: 137;
    i32 pcm_v9: 43;
    i32 pcm_v10: 1563;
    i32 fp_qs: 129;
    i32 corrhist_v1: 1074;
    i32 corrhist_v2: 919;
    i32 corrhist_v3: 724;
    i32 corrhist_v4: 1058;
    i32 corrhist_v5: 993;
    i32 corrhist_v6: 661;
    i32 corrhist_bonus_v1: 1026;
    i32 corrhist_bonus_v2: 1159;
    i32 corrhist_bonus_v3: 929;
    i32 corrhist_bonus_v4: 1129;
    i32 corrhist_bonus_v5: 1056;
    i32 corrhist_bonus_v6: 1039;
    i32 corrhist_bonus_v7: 986;
    i32 corrhist_clamp_min: 3927;
    i32 corrhist_clamp_max: 3373;
    i32 cont_v1: 1523;
    i32 cont_v2: 1144;
    i32 cont_v3: 957;
    i32 mp_v1: 1188;
    i32 mp_v2: 1028;
    i32 mp_v3: 868;
    i32 mp_v4: 473;
    i32 score_noisy_pc: 2238;
    i32 score_noisy_hist: 909;
    i32 material_base: 21682;
    i32 optimism_base: 1923;
    i32 eval_divisor: 28993;
    i32 optimims_v1: 114;
    i32 optimims_v2: 240;
    i32 alpha_raise_cap: 15;
    i32 corrhist_div: 108;
);
