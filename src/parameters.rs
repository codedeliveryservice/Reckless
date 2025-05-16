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
    i32 asp_div: 29889;
    i32 delta_base: 41;
    i32 delta_red: 14;
    i32 tthit_scale: 122;
    i32 tthit_sub: 61;
    i32 tthit_cap: 1306;
    i32 static_scale: 695;
    i32 static_min: 71;
    i32 static_max: 165;
    i32 hs_v1: 2645;
    i32 hs_v2: 979;
    i32 hs_v3: 59;
    i32 razor_offset: 262;
    i32 razor_factor: 239;
    i32 rfp_depth: 7;
    i32 rfp_base: 84;
    i32 rfp_improving: 69;
    i32 rfp_cut: 28;
    i32 rfp_correction: 492;
    i32 rfp_offset: 24;
    i32 nmp_offset: 14;
    i32 nmp_factor: 158;
    i32 nmp_base: 185;
    i32 nmp_eval_factor: 248;
    i32 probcut_base: 311;
    i32 probcut_factor: 64;
    i32 lmr_history_divisor: 6772;
    i32 fp_depth: 9;
    i32 fp_base: 95;
    i32 fp_offset: 154;
    i32 bnfp_depth: 6;
    i32 bnfp_base: 129;
    i32 bnfp_move_factor: 354;
    i32 see_quiet_factor: 20;
    i32 see_noisy_factor: 101;
    i32 see_noisy_base: 52;
    i32 see_history_factor: 41;
    i32 se_depth: 5;
    i32 se_double: 18;
    i32 se_triple: 86;
    i32 lmr_v1: 86;
    i32 lmr_v2: 566;
    i32 lmr_v3: 3678;
    i32 lmr_v4: 58;
    i32 lmr_v5: 347;
    i32 lmr_v6: 810;
    i32 lmr_v7: 558;
    i32 lmr_v8: 726;
    i32 lmr_v9: 648;
    i32 lmr_v10: 654;
    i32 lmr_v11: 29;
    i32 lmr_v12: 1169;
    i32 lmr_v13: 917;
    i32 lmr_v14: 707;
    i32 lmr_v15: 833;
    i32 lmr_v16: 66;
    i32 lmr_v17: 6;
    i32 lmr_v18: 1096;
    i32 lmr_v19: 15;
    i32 dod_base: 47;
    i32 dod_factor: 536;
    i32 post_bonus_base: 143;
    i32 post_bonus_offset: 56;
    i32 post_bonus_cap: 1172;
    i32 post_penalty_base: 127;
    i32 post_penalty_offset: 57;
    i32 post_penalty_cap: 1036;
    i32 bonus_noisy_base: 116;
    i32 bonus_noisy_offset: 63;
    i32 bonus_noisy_cap: 1159;
    i32 malus_noisy_base: 144;
    i32 malus_noisy_offset: 77;
    i32 malus_noisy_cap: 1193;
    i32 malus_noisy_move_factor: 12;
    i32 bonus_quiet_base: 145;
    i32 bonus_quiet_offset: 74;
    i32 bonus_quiet_cap: 1304;
    i32 malus_quiet_base: 125;
    i32 malus_quiet_offset: 50;
    i32 malus_quiet_cap: 1206;
    i32 malus_quiet_move_factor: 18;
    i32 bonus_cont_base: 98;
    i32 bonus_cont_offset: 57;
    i32 bonus_cont_cap: 1357;
    i32 malus_cont_base: 210;
    i32 malus_cont_offset: 57;
    i32 malus_cont_cap: 830;
    i32 malus_cont_move_factor: 15;
    i32 pcm_v1: 117;
    i32 pcm_v2: 137;
    i32 pcm_v3: 5;
    i32 pcm_v4: 214;
    i32 pcm_v5: 124;
    i32 pcm_v6: 268;
    i32 pcm_v7: 124;
    i32 pcm_v8: 140;
    i32 pcm_v9: 52;
    i32 pcm_v10: 1644;
    i32 fp_qs: 137;
    i32 corrhist_v1: 1067;
    i32 corrhist_v2: 981;
    i32 corrhist_v3: 722;
    i32 corrhist_v4: 1019;
    i32 corrhist_v5: 993;
    i32 cont_v1: 1195;
    i32 cont_v2: 1404;
    i32 cont_v3: 981;
    i32 mp_v1: 1196;
    i32 mp_v2: 1031;
    i32 mp_v3: 897;
    i32 mp_v4: 544;
    i32 material_base: 19159;
    i32 material_pawn: 140;
    i32 material_knight: 423;
    i32 material_bishop: 434;
    i32 material_rook: 631;
    i32 material_queen: 1331;
);
