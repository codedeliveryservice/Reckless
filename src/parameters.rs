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
    f32 lmp1: 4.0;
    f32 lmp2: 1.0;
    f32 lmp3: 2.0;
    f32 lmp4: 0.5;
    i32 lmp5: 17;
    i32 fp1: 107;
    i32 fp2: 48;
    i32 fp3: 90;
    i32 fp4: 75;
    i32 fp5: 8;
    i32 bnfp1: 118;
    i32 bnfp2: 96;
    i32 bnfp3: 91;
    i32 bnfp4: 67;
    i32 bnfp5: 6;
    i32 see1: 22;
    i32 see2: 32;
    i32 see3: 17;
    i32 see4: 104;
    i32 see5: 45;
    i32 see6: 46;
);
