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
    i32 quiet_fact_min: 1852;
    i32 quiet_fact_max: 1852;
    i32 quiet_bucket_min: 6324;
    i32 quiet_bucket_max: 6324;

    i32 noisy_fact_min: 4524;
    i32 noisy_fact_max: 4524;
    i32 noisy_bucket_min: 7826;
    i32 noisy_bucket_max: 7826;

    i32 contcorr_min: 16282;
    i32 contcorr_max: 16282;

    i32 cont_min: 15168;
    i32 cont_max: 15168;
);
