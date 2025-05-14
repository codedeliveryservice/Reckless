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
    i32 asp_div: 32247;
    i32 delta_base: 48;
    i32 delta_red: 16;
    i32 tthit_scale: 133;
    i32 tthit_sub: 65;
    i32 tthit_cap: 1270;
    i32 static_scale: 768;
    i32 static_min: 67;
    i32 static_max: 160;
    i32 hs_v1: 2761;
    i32 hs_v2: 1053;
    i32 hs_v3: 81;
    i32 razor_offset: 307;
    i32 razor_factor: 235;
    i32 rfp_depth: 7;
    i32 rfp_base: 80;
    i32 rfp_improving: 70;
    i32 rfp_cut: 33;
    i32 rfp_correction: 512;
    i32 rfp_offset: 25;
    i32 nmp_offset: 16;
    i32 nmp_factor: 131;
    i32 nmp_base: 205;
    i32 nmp_eval_factor: 250;
    i32 probcut_base: 302;
    i32 probcut_factor: 66;
    i32 lmr_history_divisor: 7777;
    i32 fp_depth: 9;
    i32 fp_base: 93;
    i32 fp_offset: 166;
    i32 bnfp_depth: 6;
    i32 bnfp_base: 132;
    i32 bnfp_move_factor: 384;
    i32 see_quiet_factor: 19;
    i32 see_noisy_factor: 97;
    i32 see_noisy_base: 54;
    i32 see_history_factor: 37;
    i32 se_depth: 5;
    i32 se_double: 20;
    i32 se_triple: 132;
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
    i32 lmr_v11: 32;
    i32 lmr_v12: 1105;
    i32 lmr_v13: 967;
    i32 lmr_v14: 827;
    i32 lmr_v15: 670;
    i32 lmr_v16: 63;
    i32 lmr_v17: 7;
    i32 lmr_v18: 978;
    i32 lmr_v19: 15;
    i32 dod_base: 48;
    i32 dod_factor: 512;
    i32 post_bonus_base: 143;
    i32 post_bonus_offset: 65;
    i32 post_bonus_cap: 1212;
    i32 post_penalty_base: 135;
    i32 post_penalty_offset: 71;
    i32 post_penalty_cap: 1177;
    i32 bonus_noisy_base: 133;
    i32 bonus_noisy_offset: 65;
    i32 bonus_noisy_cap: 1270;
    i32 malus_noisy_base: 143;
    i32 malus_noisy_offset: 75;
    i32 malus_noisy_cap: 1270;
    i32 malus_noisy_move_factor: 15;
    i32 bonus_quiet_base: 126;
    i32 bonus_quiet_offset: 75;
    i32 bonus_quiet_cap: 1325;
    i32 malus_quiet_base: 119;
    i32 malus_quiet_offset: 59;
    i32 malus_quiet_cap: 1180;
    i32 malus_quiet_move_factor: 18;
    i32 bonus_cont_base: 139;
    i32 bonus_cont_offset: 61;
    i32 bonus_cont_cap: 1433;
    i32 malus_cont_base: 171;
    i32 malus_cont_offset: 67;
    i32 malus_cont_cap: 1047;
    i32 malus_cont_move_factor: 16;
    i32 pcm_v1: 128;
    i32 pcm_v2: 128;
    i32 pcm_v3: 5;
    i32 pcm_v4: 256;
    i32 pcm_v5: 130;
    i32 pcm_v6: 256;
    i32 pcm_v7: 120;
    i32 pcm_v8: 146;
    i32 pcm_v9: 54;
    i32 pcm_v10: 1353;
    i32 fp_qs: 123;
    i32 corrhist_v1: 1181;
    i32 corrhist_v2: 975;
    i32 corrhist_v3: 898;
    i32 corrhist_v4: 972;
    i32 corrhist_v5: 1083;
    i32 cont_v1: 1164;
    i32 cont_v2: 1175;
    i32 cont_v3: 1035;
    i32 mp_v1: 1247;
    i32 mp_v2: 1011;
    i32 mp_v3: 978;
    i32 mp_v4: 517;
    i32 material_base: 22400;
    i32 material_pawn: 128;
    i32 material_knight: 384;
    i32 material_bishop: 416;
    i32 material_rook: 640;
    i32 material_queen: 1280;
);
