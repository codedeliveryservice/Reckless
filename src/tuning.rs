use std::sync::atomic::{AtomicI32, Ordering};

create_parameters!(
    lmr_history_divisor: 6400, range = 5000..8500, step = 150;

    bonus_multiplier: 300, range = 150..400, step = 25;
    malus_multiplier: 300, range = 150..400, step = 25;

    bonus_base: -300, range = -400..100, step = 25;
    malus_base: -300, range = -400..100, step = 25;

    bonus_max_depth: 7, range = 6..14, step = 1;
    malus_max_depth: 7, range = 6..14, step = 1;
);

pub fn set_parameter(name: &str, value: i32) {
    for v in PARAMETERS {
        if v.name == name {
            v.current.store(value, Ordering::Relaxed);
            return;
        }
    }
    eprintln!("Unknown option: {name}");
}

pub fn print_options() {
    for v in PARAMETERS {
        let current = v.current.load(Ordering::Relaxed);
        println!("option name {} type spin default {current} min {} max {}", v.name, v.min, v.max);
    }
}

pub fn print_parameters() {
    for v in PARAMETERS {
        let current = v.current.load(Ordering::Relaxed);
        println!("{}, int, {current}, {}, {}, {}, 0.002", v.name, v.min, v.max, v.step);
    }
}

pub struct Parameter {
    name: &'static str,
    current: AtomicI32,
    min: i32,
    max: i32,
    step: i32,
}

macro_rules! create_parameters {
    ( $($name:ident: $current:expr, range = $range:expr, step = $step:expr;)* ) => {
        static PARAMETERS: &[&Parameter] = &[$(&parameters::$name),*];

        $(
            pub fn $name() -> i32 {
                parameters::$name.current.load(std::sync::atomic::Ordering::Relaxed)
            }
        )*

        mod parameters {
            use super::Parameter;

            $(
                #[allow(non_upper_case_globals)]
                pub static $name: Parameter = Parameter {
                    name: stringify!($name),
                    current: std::sync::atomic::AtomicI32::new($current),
                    min: $range.start,
                    max: $range.end,
                    step: $step,
                };
            )*
        }
    };
}

use create_parameters;
