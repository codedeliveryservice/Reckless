pub const PIECE_VALUES: [i32; 7] = [109, 403, 435, 679, 1242, 0, 0];

#[allow(unused_macros)]
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
    i32 ttcut_quiet_v1: 141;
    i32 ttcut_quiet_v2: 72;
    i32 ttcut_quiet_v3: 1544;

    i32 ttcut_cont_v1: 99;
    i32 ttcut_cont_v2: 61;
    i32 ttcut_cont_v3: 1509;

    i32 bonus_noisy_v1a: 125;
    i32 bonus_noisy_v1b: 125;
    i32 bonus_noisy_v2a: 111;
    i32 bonus_noisy_v2b: 111;
    i32 bonus_noisy_v3a: 1175;
    i32 bonus_noisy_v3b: 1175;

    i32 malus_noisy_v1a: 153;
    i32 malus_noisy_v1b: 153;
    i32 malus_noisy_v2a: 64;
    i32 malus_noisy_v2b: 64;
    i32 malus_noisy_v3a: 1476;
    i32 malus_noisy_v3b: 1476;

    i32 bonus_quiet_v1a: 152;
    i32 bonus_quiet_v1b: 152;
    i32 bonus_quiet_v2a: 123;
    i32 bonus_quiet_v2b: 123;
    i32 bonus_quiet_v3a: 1569;
    i32 bonus_quiet_v3b: 1569;

    i32 malus_quiet_v1a: 133;
    i32 malus_quiet_v1b: 133;
    i32 malus_quiet_v2a: 51;
    i32 malus_quiet_v2b: 51;
    i32 malus_quiet_v3a: 1162;
    i32 malus_quiet_v3b: 1162;

    i32 bonus_cont_v1a: 102;
    i32 bonus_cont_v1b: 102;
    i32 bonus_cont_v2a: 107;
    i32 bonus_cont_v2b: 107;
    i32 bonus_cont_v3a: 1223;
    i32 bonus_cont_v3b: 1223;

    i32 malus_cont_v1a: 306;
    i32 malus_cont_v1b: 306;
    i32 malus_cont_v2a: 46;
    i32 malus_cont_v2b: 46;
    i32 malus_cont_v3a: 1018;
    i32 malus_cont_v3b: 1018;

    i32 refuted_v1a: 78;
    i32 refuted_v1b: 78;
    i32 refuted_v2a: 52;
    i32 refuted_v2b: 52;
    i32 refuted_v3a: 811;
    i32 refuted_v3b: 811;

    i32 pcm_v1a: 156;
    i32 pcm_v1b: 156;
    i32 pcm_v2a: 42;
    i32 pcm_v2b: 42;
    i32 pcm_v3a: 1789;
    i32 pcm_v3b: 1789;

    i32 pcm2_v1a: 151;
    i32 pcm2_v1b: 151;
    i32 pcm2_v2a: 41;
    i32 pcm2_v2b: 41;
    i32 pcm2_v3a: 1630;
    i32 pcm2_v3b: 1630;
);
