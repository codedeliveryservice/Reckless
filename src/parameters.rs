pub const SEE_PIECE_VALUES: [i32; 7] = [100, 400, 400, 650, 1200, 0, 0];
pub const OPT_PIECE_VALUES: [i32; 7] = [400, 750, 800, 1200, 1900, 0, 0];

pub const LMR_MOVES_PLAYED: i32 = 3;
pub const LMR_DEPTH: i32 = 3;

pub const LMP_DEPTH: i32 = 4;
pub const LMP_MARGIN: i32 = 3;

pub struct Parameters {
    lmr: [[f64; 64]; 64],
}

impl Parameters {
    pub fn lmr(&self, depth: i32, moves: i32) -> f64 {
        self.lmr[depth.min(63) as usize][moves.min(63) as usize]
    }
}

impl Default for Parameters {
    fn default() -> Self {
        let mut lmr = [[0.0; 64]; 64];
        for (depth, row) in lmr.iter_mut().enumerate() {
            for (moves, r) in row.iter_mut().enumerate() {
                *r = lmr_base() + (depth as f64).ln() * (moves as f64).ln() / lmr_divisor();
            }
        }
        Self { lmr }
    }
}

#[cfg(not(feature = "tuning"))]
macro_rules! define {
    ($($type:ident $name:ident: $value:expr, $min:expr, $max:expr; )*) => {
        $(pub const fn $name() -> $type {
            $value
        })*
    };
}

#[cfg(feature = "tuning")]
macro_rules! define {
    ($($type:ident $name:ident: $value:expr, $min:expr, $max:expr; )*) => {
        pub fn set_parameter(name: &str, value: &str) {
            match name {
                $(stringify!($name) => unsafe { parameters::$name = value.parse().unwrap() },)*
                _ => panic!("Unknown tunable parameter: {}", name),
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

define!(
    i32 rfp_depth: 7, 1, 15;
    i32 rfp_margin: 75, 0, 150;

    i32 razoring_depth: 4, 1, 10;
    i32 razoring_margin: 220, 0, 440;
    i32 razoring_fixed_margin: 135, 0, 270;

    i32 fp_depth: 5, 1, 10;
    i32 fp_margin: 130, 0, 260;
    i32 fp_fixed_margin: 45, 0, 90;

    i32 search_deeper_margin: 80, 0, 160;

    i32 see_depth: 6, 1, 112;
    i32 see_noisy_margin: 100, 50, 150;
    i32 see_quiet_margin: 70, 50, 150;

    i32 iir_depth: 4, 1, 10;

    i32 aspiration_depth: 6, 1, 12;
    i32 aspiration_delta: 30, 15, 45;

    f64 lmr_base: 0.73, 0.5, 1.5;
    f64 lmr_divisor: 2.22, 1.5, 3.5;

    i32 lmr_history: 6210, 4000, 8000;

    i32 history_bonus: 130, 0, 400;
    i32 history_bonus_base: -30, -150, 150;
    i32 history_bonus_max: 1790, 1000, 3500;

    i32 history_malus: 180, 0, 400;
    i32 history_malus_base: 20, -150, 150;
    i32 history_malus_max: 1640, 1000, 3500;
);
