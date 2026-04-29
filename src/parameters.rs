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
    f64 soft1: 0.066;
    f64 soft2: 0.042;
    f64 soft3: 0.045;
    f64 hard1: 0.742;
    f32 nodes1: 2.7168;
    f32 nodes2: 2.2669;
    f32 nodes3: 0.5630;
    f32 score1: 0.80;
    f32 score2: 0.05;
    f32 score3: 0.80;
    f32 score4: 1.45;
    f32 pv1: 1.25;
    f32 pv2: 0.05;
    f32 pv3: 0.85;
    f32 eval1: 1.2;
    f32 eval2: 0.04;
    f32 eval3: 0.88;
    f32 bm1: 1.0;
    f32 bm2: 0.25;
);
