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
    i32 razoring1: -252;
    i32 razoring2: 0;
    i32 razoring3: -299;

    i32 rfp1: -10;
    i32 rfp2: 1385;
    i32 rfp3: 1125;
    i32 rfp4: 5125;

    i32 nmp1: 0;
    i32 nmp2: -9;
    i32 nmp3: 286;
    i32 nmp4: 0;
    i32 nmp5: 271;
    i32 nmp6: 5154;

    i32 lmp1: 1075;
    i32 lmp2: 0;
    i32 lmp3: 3127;
    i32 lmp4: 311;
    i32 lmp5: 0;
    i32 lmp6: 1320;

    i32 fp1: 0;
    i32 fp2: 88;
    i32 fp3: -114;

    i32 bnfp1: 0;
    i32 bnfp2: 71;
    i32 bnfp3: 25;

    i32 see1: -16;
    i32 see2: 52;
    i32 see3: 22;
    i32 see4: -8;
    i32 see5: -36;
    i32 see6: 11;
);
