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
    i32 raz1: 551;
    i32 raz2: 1307;
    i32 raz3: 2567;
    i32 raz4: 4331;
    i32 raz5: 6599;
    i32 raz6: 9371;
    i32 raz7: 12647;
    i32 raz8: 16427;
    i32 rfp1: 66;
    i32 rfp2: 87;
    i32 rfp3: 157;
    i32 rfp4: 244;
    i32 rfp5: 349;
    i32 rfp6: 472;
    i32 rfp7: 612;
    i32 rfp8: 770;
    i32 rfp9: 945;
    i32 rfp10: 1138;
    i32 rfp11: 1349;
    i32 rfp12: 1577;
    i32 rfp13: 1823;
    i32 rfp14: 2086;
    i32 rfp15: 2367;
    i32 nmp1: 277;
    i32 nmp2: 268;
    i32 nmp3: 259;
    i32 nmp4: 250;
    i32 nmp5: 241;
    i32 nmp6: 232;
    i32 nmp7: 223;
    i32 nmp8: 214;
    i32 nmp9: 205;
    i32 nmp10: 196;
    i32 nmp11: 187;
    i32 nmp12: 178;
    i32 nmp13: 169;
    i32 nmp14: 160;
    i32 nmp15: 151;
    i32 nmp16: 142;
    i32 nmp17: 133;
    i32 nmp18: 124;
    i32 nmp19: 115;
    i32 nmp20: 106;
    i32 nmp21: 97;
    i32 nmp22: 88;
    i32 nmp23: 79;
    i32 nmp24: 70;
    i32 nmp25: 61;
    i32 nmp26: 52;
    i32 nmp27: 43;
    i32 nmp28: 34;
    i32 nmp29: 25;
    i32 nmp30: 16;
    i32 nmp31: 7;
    i32 fp1: -26;
    i32 fp2: 62;
    i32 fp3: 150;
    i32 fp4: 238;
    i32 fp5: 326;
    i32 fp6: 414;
    i32 fp7: 502;
    i32 fp8: 590;
    i32 fp9: 678;
    i32 fp10: 766;
    i32 fp11: 854;
    i32 fp12: 942;
    i32 fp13: 1030;
    i32 bnfp1: 96;
    i32 bnfp2: 167;
    i32 bnfp3: 238;
    i32 bnfp4: 309;
    i32 bnfp5: 380;
    i32 bnfp6: 451;
    i32 bnfp7: 522;
    i32 bnfp8: 593;
    i32 bnfp9: 664;
    i32 bnfp10: 735;
    i32 bnfp11: 806;
    i32 see_quiet1: 58;
    i32 see_quiet2: 62;
    i32 see_quiet3: 34;
    i32 see_quiet4: -26;
    i32 see_quiet5: -118;
    i32 see_quiet6: -242;
    i32 see_quiet7: -398;
    i32 see_quiet8: -586;
    i32 see_quiet9: -806;
    i32 see_quiet10: -1058;
    i32 see_quiet11: -1342;
    i32 see_quiet12: -1658;
    i32 see_quiet13: -2006;
    i32 see_quiet14: -2386;
    i32 see_quiet15: -2798;
    i32 see_quiet16: -3242;
    i32 see_noisy1: -33;
    i32 see_noisy2: -93;
    i32 see_noisy3: -169;
    i32 see_noisy4: -261;
    i32 see_noisy5: -369;
    i32 see_noisy6: -493;
    i32 see_noisy7: -633;
    i32 see_noisy8: -789;
    i32 see_noisy9: -961;
    i32 see_noisy10: -1149;
    i32 see_noisy11: -1353;
    i32 see_noisy12: -1573;
    i32 see_noisy13: -1809;
    i32 see_noisy14: -2061;
    i32 see_noisy15: -2329;
    i32 see_noisy16: -2613;
);
