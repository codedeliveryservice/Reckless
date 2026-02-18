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
    f32 nodes_a: 2.7168;
    f32 nodes_b: 2.2669;
    f32 nodes_min: 0.563;

    f32 pv_a: 1.25;
    f32 pv_b: 0.05;
    f32 pv_min: 0.85;

    f32 eval_a: 1.2;
    f32 eval_b: 0.04;
    f32 eval_min: 0.88;

    f32 score_a: 0.8;
    f32 score_b: 0.05;
    f32 score_clamp_min: 0.80;
    f32 score_clamp_max: 1.45;

    f32 best_base: 1.0;
    f32 best_div: 4.0;
);
