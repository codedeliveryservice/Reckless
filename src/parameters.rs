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
    i32 razoring1: -243;
    i32 razoring2: -6;
    i32 razoring3: -294;

    i32 rfp1: -10;
    i32 rfp2: 1306;
    i32 rfp3: 1115;
    i32 rfp4: 5485;

    i32 nmp1: 0;
    i32 nmp2: -9;
    i32 nmp3: 297;
    i32 nmp4: 2;
    i32 nmp5: 261;
    i32 nmp6: 4955;

    i32 lmp1: 1073;
    i32 lmp2: 68;
    i32 lmp3: 3067;
    i32 lmp4: 326;
    i32 lmp5: 5;
    i32 lmp6: 1358;

    i32 fp1: 1;
    i32 fp2: 95;
    i32 fp3: -123;

    i32 bnfp1: 1;
    i32 bnfp2: 69;
    i32 bnfp3: 22;

    i32 see1: -16;
    i32 see2: 53;
    i32 see3: 11;
    i32 see4: -9;
    i32 see5: -36;
    i32 see6: 9;
);
