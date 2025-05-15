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

define!(
    i32 asp_delta: 10;
    i32 asp_div: 27874;
    i32 delta_base: 45;
    i32 delta_red: 14;
    i32 tthit_scale: 124;
    i32 tthit_sub: 64;
    i32 tthit_cap: 1367;
    i32 static_scale: 664;
    i32 static_min: 69;
    i32 static_max: 164;
    i32 hs_v1: 2551;
    i32 hs_v2: 1014;
    i32 hs_v3: 67;
    i32 razor_offset: 268;
    i32 razor_factor: 250;
    i32 rfp_depth: 7;
    i32 rfp_base: 82;
    i32 rfp_improving: 69;
    i32 rfp_cut: 27;
    i32 rfp_correction: 531;
    i32 rfp_offset: 24;
    i32 nmp_offset: 15;
    i32 nmp_factor: 153;
    i32 nmp_base: 190;
    i32 nmp_eval_factor: 252;
    i32 probcut_base: 298;
    i32 probcut_factor: 64;
    i32 lmr_history_divisor: 7084;
    i32 fp_depth: 9;
    i32 fp_base: 97;
    i32 fp_offset: 175;
    i32 bnfp_depth: 6;
    i32 bnfp_base: 122;
    i32 bnfp_move_factor: 371;
    i32 see_quiet_factor: 21;
    i32 see_noisy_factor: 98;
    i32 see_noisy_base: 50;
    i32 see_history_factor: 42;
    i32 se_depth: 5;
    i32 se_double: 17;
    i32 se_triple: 97;
    i32 lmr_v1: 90;
    i32 lmr_v2: 556;
    i32 lmr_v3: 3819;
    i32 lmr_v4: 57;
    i32 lmr_v5: 313;
    i32 lmr_v6: 792;
    i32 lmr_v7: 597;
    i32 lmr_v8: 717;
    i32 lmr_v9: 622;
    i32 lmr_v10: 619;
    i32 lmr_v11: 32;
    i32 lmr_v12: 1141;
    i32 lmr_v13: 928;
    i32 lmr_v14: 749;
    i32 lmr_v15: 770;
    i32 lmr_v16: 62;
    i32 lmr_v17: 7;
    i32 lmr_v18: 1043;
    i32 lmr_v19: 15;
    i32 dod_base: 46;
    i32 dod_factor: 542;
    i32 post_bonus_base: 138;
    i32 post_bonus_offset: 54;
    i32 post_bonus_cap: 1223;
    i32 post_penalty_base: 124;
    i32 post_penalty_offset: 60;
    i32 post_penalty_cap: 1172;
    i32 bonus_noisy_base: 128;
    i32 bonus_noisy_offset: 60;
    i32 bonus_noisy_cap: 1132;
    i32 malus_noisy_base: 141;
    i32 malus_noisy_offset: 75;
    i32 malus_noisy_cap: 1173;
    i32 malus_noisy_move_factor: 14;
    i32 bonus_quiet_base: 129;
    i32 bonus_quiet_offset: 75;
    i32 bonus_quiet_cap: 1370;
    i32 malus_quiet_base: 126;
    i32 malus_quiet_offset: 48;
    i32 malus_quiet_cap: 1245;
    i32 malus_quiet_move_factor: 18;
    i32 bonus_cont_base: 115;
    i32 bonus_cont_offset: 58;
    i32 bonus_cont_cap: 1357;
    i32 malus_cont_base: 204;
    i32 malus_cont_offset: 60;
    i32 malus_cont_cap: 911;
    i32 malus_cont_move_factor: 15;
    i32 pcm_v1: 118;
    i32 pcm_v2: 134;
    i32 pcm_v3: 5;
    i32 pcm_v4: 211;
    i32 pcm_v5: 135;
    i32 pcm_v6: 273;
    i32 pcm_v7: 118;
    i32 pcm_v8: 140;
    i32 pcm_v9: 51;
    i32 pcm_v10: 1555;
    i32 fp_qs: 131;
    i32 corrhist_v1: 1114;
    i32 corrhist_v2: 975;
    i32 corrhist_v3: 757;
    i32 corrhist_v4: 1015;
    i32 corrhist_v5: 992;
    i32 cont_v1: 1287;
    i32 cont_v2: 1323;
    i32 cont_v3: 937;
    i32 mp_v1: 1141;
    i32 mp_v2: 1031;
    i32 mp_v3: 988;
    i32 mp_v4: 554;
    i32 material_base: 20099;
    i32 material_pawn: 132;
    i32 material_knight: 414;
    i32 material_bishop: 432;
    i32 material_rook: 661;
    i32 material_queen: 1217;
);
