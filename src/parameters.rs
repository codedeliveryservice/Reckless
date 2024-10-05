pub const SEE_PIECE_VALUES: [i32; 7] = [100, 400, 400, 650, 1200, 0, 0];
pub const OPT_PIECE_VALUES: [i32; 7] = [400, 750, 800, 1200, 1900, 0, 0];

pub const LMR_MOVES_PLAYED: i32 = 3;
pub const LMR_DEPTH: i32 = 3;
pub const LMR_BASE: f64 = 0.73;
pub const LMR_DIVISOR: f64 = 2.22;
pub const LMR_HISTORY_DIVISOR: f64 = 6210.0;

pub const LMP_DEPTH: i32 = 4;
pub const LMP_MARGIN: i32 = 3;

pub const IIR_DEPTH: i32 = 4;

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
        let mut lmr = [[0f64; 64]; 64];
        for (depth, row) in lmr.iter_mut().enumerate() {
            for (moves, r) in row.iter_mut().enumerate() {
                *r = LMR_BASE + (depth as f64).ln() * (moves as f64).ln() / LMR_DIVISOR;
            }
        }
        Self { lmr }
    }
}

#[cfg(not(feature = "tuning"))]
macro_rules! define {
    ($($name:ident: $value:expr, $range:expr; )*) => {$(
        pub const fn $name() -> i32 {
            $value
        }
    )*};
}

#[cfg(feature = "tuning")]
macro_rules! define {
    ($($name:ident: $value:expr, $range:expr; )*) => {
        use std::sync::atomic::{AtomicI32, Ordering};

        static PARAMETERS: &[&Parameter] = &[$(&parameters::$name),*];

        pub fn set_parameter(name: &str, value: i32) {
            match PARAMETERS.iter().find(|p| p.name == name) {
                Some(p) => p.value.store(value, Ordering::Relaxed),
                None => panic!("Unknown tunable parameters: {name}"),
            }
        }

        pub fn print_options() {
            for v in PARAMETERS {
                let current = v.value.load(Ordering::Relaxed);
                println!("option name {} type spin default {current} min {} max {}", v.name, v.min, v.max);
            }
        }

        pub fn print_parameters() {
            for v in PARAMETERS {
                let step = (v.max - v.min) as f64 / 15.0;
                let step = if step < 1.0 { 0.5 } else { step.round() };

                println!("{}, int, {}, {}, {}, {step}, 0.002", v.name, v.value.load(Ordering::Relaxed), v.min, v.max);
            }
        }

        pub struct Parameter {
            name: &'static str,
            value: AtomicI32,
            min: i32,
            max: i32,
        }

        $(
            pub fn $name() -> i32 {
                parameters::$name.value.load(std::sync::atomic::Ordering::Relaxed)
            }
        )*

        mod parameters {
            use super::Parameter;

            $(
                #[allow(non_upper_case_globals)]
                pub static $name: Parameter = Parameter {
                    name: stringify!($name),
                    value: std::sync::atomic::AtomicI32::new($value),
                    min: $range.start,
                    max: $range.end,
                };
            )*
        }
    };
}

define!(
    rfp_margin:             75, 0..150;
    rfp_depth:               7, 1..15;

    razoring_depth:          4, 1..10;
    razoring_margin:       220, 0..440;
    razoring_fixed_margin: 135, 0..270;

    fp_depth:                5, 1..10;
    fp_margin:             130, 0..260;
    fp_fixed_margin:        45, 0..90;

    search_deeper_margin:   80, 0..160;

    see_depth:               6, 1..112;
    see_noisy_margin:      100, 50..150;
    see_quiet_margin:       70, 50..150;

    aspiration_depth:        6, 1..12;
    aspiration_delta:       30, 15..45;
);
