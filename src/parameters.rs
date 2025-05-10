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
    i32 hist_mp_0: 1141;
    i32 conthist_mp_1: 1031;
    i32 conthist_mp_2: 988;
    i32 conthist_mp_3: 554;
    i32 conthist_mp_4: 1024;
    i32 conthist_mp_5: 1024;
    i32 conthist_mp_6: 1024;

    i32 hist_lmr_0: 1024;
    i32 conthist_lmr_1: 1024;
    i32 conthist_lmr_2: 1024;
    i32 conthist_lmr_3: 0;
    i32 conthist_lmr_4: 0;
    i32 conthist_lmr_5: 0;
    i32 conthist_lmr_6: 0;

    i32 conthist_s_1: 1287;
    i32 conthist_s_2: 1323;
    i32 conthist_s_3: 937;
    i32 conthist_s_4: 1024;
    i32 conthist_s_5: 1024;
    i32 conthist_s_6: 1024;
    i32 prun_hist_div: 7084;
    i32 red_hist_mul: 90;
    i32 red_hist_cor: 556;
);
