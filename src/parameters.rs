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
    i32 eval1: 21366;
    i32 eval2: 1747;
    i32 eval3: 27395;
    i32 hist1: 82;
    i32 hist2: 105;
    i32 max1: 1940;
    i32 max2: 6029;
    i32 max3: 4449;
    i32 max4: 8148;
    i32 max5: 14734;
    i32 max6: 16222;
    i32 max7: 15324;
    i32 asp1: 24616;
    i32 asp2: 30;
    i32 asp3: 50;
    i32 asp4: 10;
    i32 opt1: 119;
    i32 opt2: 237;
    i32 cut1: 141;
    i32 cut2: 72;
    i32 cut3: 1544;
    i32 cut4: 68;
    i32 cut5: 99;
    i32 cut6: 61;
    i32 cut7: 1509;
    i32 cut8: 65;
    i32 ord1: 733;
    i32 ord2: 123;
    i32 ord3: 255;
    i32 hs1: 2397;
    i32 hs2: 0;
    i32 hs3: 963;
    i32 hs4: 63;
    i32 razor1: 320;
    i32 razor2: 237;
    i32 serfp1: 75;
    i32 serfp2: 85;
    i32 serfp3: 580;
    i32 rfp1: 157;
    i32 rfp2: 31;
    i32 rfp3: 71;
    i32 rfp4: 23;
    i32 rfp5: 580;
    i32 rfp6: 24;
    i32 nmp1: 16;
    i32 nmp2: 158;
    i32 nmp3: 106;
    i32 nmp4: 213;
    i32 nmp5: 5756;
    i32 nmp6: 321;
    i32 probcut1: 259;
    i32 probcut2: 65;
    i32 lmp1: 17;
    i32 lmp2: 3728;
    i32 lmp3: 998;
    i32 lmp4: 1904;
    i32 lmp5: 470;
    i32 fp1: 105;
    i32 fp2: 49;
    i32 fp3: 95;
    i32 fp4: 83;
    i32 bnfp1: 123;
    i32 bnfp2: 72;
    i32 bnfp3: 94;
    i32 bnfp4: 71;
    i32 see1: 325;
    i32 see2: 31;
    i32 see3: 16;
    i32 see4: 102;
    i32 see5: 45;
    i32 see6: 46;
    i32 se1: 277;
    i32 se2: 67;
    i32 se3: 315;
    i32 se4: 16;
    i32 lmr1: 489;
    i32 lmr2: 412;
    i32 lmr3: 1243;
    i32 lmr4: 489;
    i32 lmr5: 137;
    i32 lmr6: 488;
    i32 lmr7: 109;
    i32 lmr8: 46;
    i32 lmr9: 3607;
    i32 lmr10: 69;
    i32 lmr11: 427;
    i32 lmr12: 677;
    i32 lmr13: 729;
    i32 lmr14: 393;
    i32 lmr15: 552;
    i32 lmr16: 1675;
    i32 lmr17: 934;
    i32 lmr18: 1049;
    i32 lmr19: 1555;
    i32 lmr20: 791;
    i32 lmr21: 1397;
    i32 dod1: 37;
    i32 dod2: 495;
    i32 post1: 155;
    i32 post2: 63;
    i32 post3: 851;
    i32 fds1: 380;
    i32 fds2: 153;
    i32 fds3: 355;
    i32 fds4: 68;
    i32 fds5: 47;
    i32 fds6: 2667;
    i32 fds7: 52;
    i32 fds8: 750;
    i32 fds9: 537;
    i32 fds10: 1081;
    i32 fds11: 491;
    i32 fds12: 780;
    i32 fds13: 1478;
    i32 fds14: 1048;
    i32 fds15: 744;
    i32 fds16: 1438;
    i32 fds17: 849;
    i32 fds18: 1052;
    i32 fds19: 3034;
    i32 bonus1: 125;
    i32 bonus2: 57;
    i32 bonus3: 1175;
    i32 bonus4: 70;
    i32 malus1: 153;
    i32 malus2: 64;
    i32 malus3: 1476;
    i32 malus4: 24;
    i32 bonus5: 152;
    i32 bonus6: 73;
    i32 bonus7: 1569;
    i32 bonus8: 64;
    i32 malus5: 133;
    i32 malus6: 51;
    i32 malus7: 1162;
    i32 malus8: 37;
    i32 bonus9: 102;
    i32 bonus10: 56;
    i32 bonus11: 1223;
    i32 bonus12: 65;
    i32 malus9: 306;
    i32 malus10: 46;
    i32 malus11: 1018;
    i32 malus12: 30;
    i32 upd1: 78;
    i32 upd2: 52;
    i32 upd3: 811;
    i32 pcm1: 104;
    i32 pcm2: 147;
    i32 pcm3: 217;
    i32 pcm4: 132;
    i32 pcm5: 297;
    i32 pcm6: 100;
    i32 pcm7: 156;
    i32 pcm8: 42;
    i32 pcm9: 1789;
    i32 pcm10: 151;
    i32 pcm11: 41;
    i32 pcm12: 1630;
    i32 corr1: 150;
    i32 corr2: 4194;
    i32 corr3: 3164;
    i32 qs1: 79;
    i32 qs2: 32;
    f32 red1: 970.0027;
    f32 red2: 457.7087;
);
