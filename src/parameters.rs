pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

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
    f64 soft1: 0.024;
    f64 soft2: 0.042;
    f64 soft3: 0.045;
    f64 hard1: 0.135;
    f64 hard2: 0.145;
    f64 hard3: 0.043;
    f32 nodes1: 2.15;
    f32 nodes2: 1.5;
    f32 pv1: 1.25;
    f32 pv2: 0.05;
    f32 eval1: 1.2;
    f32 eval2: 0.04;
    f32 score1: 0.8;
    f32 score2: 0.02;
    f32 score3: 0.75;
    f32 score4: 1.5;
    i32 pv_stability_max: 8;
    i32 eval_stability_max: 8;
    i32 eval_stability_delta: 12;
);
