pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

pub fn lmp_threshold(depth: i32, improving: bool) -> i32 {
    (3 + depth * depth) / (2 - improving as i32)
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
            $(println!("option name {} type string default {}", stringify!($name), $value);)*
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
    i32 asp_step:  16;

    i32 razor_v1: 300;
    i32 razor_v2: 250;

    i32 rfp_v1: 8;
    i32 rfp_v2: 80;
    i32 rfp_v3: 80;

    i32 nmp_v1: 20;
    i32 nmp_v2: 128;
    i32 nmp_v3: 180;
    i32 nmp_v4: 256;

    i32 probcut_v1: 256;
    i32 probcut_v2: 64;

    i32 fp_v1: 10;
    i32 fp_v2: 100;
    i32 fp_v3: 150;

    i32 see_v1: 30;
    i32 see_v2: 95;

    i32 se_v1: 24;
    i32 se_v2: 128;

    i32 ch_v1: 1024;
    i32 ch_v2: 1024;
    i32 ch_v3: 1024;
    i32 ch_v4: 1024;
    i32 ch_v5: 1024;
    i32 ch_v6: 1024;

    i32 ch_v7: 8;
    i32 ch_v8: 8;
    i32 ch_v9: 96;

    i32 noisy_v1: 128;
    i32 noisy_v2: 64;
    i32 noisy_v3: 1280;

    i32 noisy_v4: 128;
    i32 noisy_v5: 64;
    i32 noisy_v6: 1280;

    i32 quiet_v1: 128;
    i32 quiet_v2: 64;
    i32 quiet_v3: 1280;

    i32 quiet_v4: 128;
    i32 quiet_v5: 64;
    i32 quiet_v6: 1280;

    i32 cnht_v1: 128;
    i32 cnht_v2: 64;
    i32 cnht_v3: 1280;

    i32 cnht_v4: 128;
    i32 cnht_v5: 64;
    i32 cnht_v6: 1280;

    i32 cnht_v7: 128;
    i32 cnht_v8: 64;
    i32 cnht_v9: 1280;

    i32 cnht_v10: 128;
    i32 cnht_v11: 64;
    i32 cnht_v12: 1280;

    i32 lmr_v1: 4096;
    i32 lmr_v2: 512;
    i32 lmr_v3: 16384;
    i32 lmr_v4: 1024;
    i32 lmr_v5: 768;
    i32 lmr_v6: 768;
    i32 lmr_v7: 1024;
    i32 lmr_v8: 1024;
    i32 lmr_v9: 1024;

    i32 dod_v1: 64;
}
