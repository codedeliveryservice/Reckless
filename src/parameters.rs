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
    f32 nodes1: 2.15;
    f32 nodes2: 1.5;
    f32 nodes3: 0.75;
    f32 nodes4: 1.75;
    f32 pv1: 1.25;
    f32 pv2: 0.05;
    f32 pv3: 0.85;
    f32 eval1: 1.2;
    f32 eval2: 0.04;
    f32 eval3: 0.88;
    f32 trend1: 0.8;
    f32 trend2: 0.05;
    f32 trend3: 0.8;
    f32 trend4: 1.45;    
    f32 recap: 0.9;
    f32 bmc1: 1.0;
    f32 bmc2: 4.0;
    f32 bmc3: 3.0;
    f64 soft1: 0.024;
    f64 soft2: 0.042;
    f64 soft3: 0.045;    
    f64 hard1: 0.742;
);
