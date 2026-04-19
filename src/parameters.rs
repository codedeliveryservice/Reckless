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
    i32 rfp1: 2971;
    i32 rfp2: 1116;
    i32 rfp3: 8182;
    i32 rfp4: 70;
    i32 rfp5: 10;
    i32 rfp6: 2861;
    i32 rfp7: 3990;
    i32 rfp8: 6826;
    i32 nmp1: 964;
    i32 nmp2: 964;
    i32 nmp3: 32;
    i32 nmp4: 14;
    i32 nmp5: 2882;
    i32 nmp6: 1691;
    i32 nmp7: 38772;
);
