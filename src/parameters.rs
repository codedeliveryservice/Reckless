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
    i32 asp_delta: 12;
    i32 asp_div:   32768;

    i32 eval_history: 6;
    i32 eval_history_min: 64;
    i32 eval_history_max: 128;

    i32 retro_v1: 3072;
    i32 retro_v2: 1024;
    i32 retro_v3: 96;

    i32 razor_v1: 300;
    i32 razor_v2: 250;

    i32 rfp_v1: 8;
    i32 rfp_v2: 80;
    i32 rfp_v3: 80;
    i32 rfp_v4: 30;
    i32 rfp_v5: 512;
    i32 rfp_v6: 25;

    i32 nmp_v1: 20;
    i32 nmp_v2: 128;
    i32 nmp_v3: 180;
    i32 nmp_v4: 256;
    i32 nmp_v5: 3;

    i32 probcut_v1: 256;
    i32 probcut_v2: 64;

    i32 fp_v1: 10;
    i32 fp_v2: 100;
    i32 fp_v3: 150;

    i32 bnp_v1: 6;
    i32 bnp_v2: 128;

    i32 see_v1: 30;
    i32 see_v2: 95;
    i32 see_v3: 50;
    i32 see_v4: 32;

    i32 se_v1: 8;
    i32 se_v2: 24;
    i32 se_v3: 128;

    i32 lmr_v1: 64;
    i32 lmr_v2: 512;
    i32 lmr_v3: 4096;
    i32 lmr_v4: 64;
    i32 lmr_v5: 256;
    i32 lmr_v6: 768;
    i32 lmr_v7: 768;
    i32 lmr_v8: 768;
    i32 lmr_v9: 768;
    i32 lmr_v10: 768;
    i32 lmr_v11: 1024;
    i32 lmr_v12: 1024;
    i32 lmr_v13: 1024;
    i32 lmr_v14: 896;
    i32 lmr_v15: 64;
    i32 lmr_v16: 8;
    i32 lmr_v17: 1024;

    i32 dod: 64;

    i32 post_v1: 128;
    i32 post_v2: 64;
    i32 post_v3: 1280;
    i32 post_v4: 128;
    i32 post_v5: 64;
    i32 post_v6: 1280;

    i32 pv_v1: 16;

    i32 bonus_noisy_v1: 128;
    i32 bonus_noisy_v2: 64;
    i32 bonus_noisy_v3: 1280;

    i32 malus_noisy_v1: 128;
    i32 malus_noisy_v2: 64;
    i32 malus_noisy_v3: 1280;
    i32 malus_noisy_v4: 16;

    i32 bonus_quiet_v1: 128;
    i32 bonus_quiet_v2: 64;
    i32 bonus_quiet_v3: 1280;

    i32 malus_quiet_v1: 128;
    i32 malus_quiet_v2: 64;
    i32 malus_quiet_v3: 1280;
    i32 malus_quiet_v4: 16;

    i32 bonus_cont_v1: 128;
    i32 bonus_cont_v2: 64;
    i32 bonus_cont_v3: 1280;

    i32 malus_cont_v1: 128;
    i32 malus_cont_v2: 64;
    i32 malus_cont_v3: 1280;
    i32 malus_cont_v4: 16;

    i32 pcm_v1: 5;
    i32 pcm_v2: 128;
    i32 pcm_v3: 128;
    i32 pcm_v4: 128;
    i32 pcm_v5: 64;
    i32 pcm_v6: 1280;

    i32 qs_v1: 128;

    i32 corrhist_v1: 1024;
    i32 corrhist_v2: 1024;
    i32 corrhist_v3: 1024;
    i32 corrhist_v4: 1024;
    i32 corrhist_v5: 1024;

    i32 conthist_v1: 1024;
    i32 conthist_v2: 1024;
    i32 conthist_v3: 1024;

    i32 mp_v1: 32;
    i32 mp_v2: 100;
    i32 mp_v3: 16;
    i32 mp_v4: 1024;
    i32 mp_v5: 1024;
    i32 mp_v6: 1024;
    i32 mp_v7: 512;
}
