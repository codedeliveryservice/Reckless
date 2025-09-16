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
    i32 eval1: 21063;
    i32 eval2: 1817;
    i32 eval3: 27627;
    i32 corr4: 81;
    i32 corr5: 104;
    i32 mp1: 36;
    i32 mp2: 116;
    i32 mp3: 2009;
    i32 mp4: 1067;
    i32 delta1: 12;
    i32 delta2: 26614;
    i32 opt1: 118;
    i32 opt2: 237;
    i32 delta3: 38;
    i32 delta4: 15;
    i32 hist1: 144;
    i32 hist2: 74;
    i32 hist3: 1466;
    i32 hist4: 67;
    i32 hist5: 92;
    i32 hist6: 61;
    i32 hist7: 1493;
    i32 hist8: 65;
    i32 hist9: 722;
    i32 hist10: 120;
    i32 hist11: 256;
    i32 hr1: 2561;
    i32 hr2: 980;
    i32 hr3: 61;
    i32 raz1: 305;
    i32 raz2: 239;
    i32 rfp1: 10;
    i32 rfp2: 30;
    i32 rfp3: 72;
    i32 rfp4: 23;
    i32 rfp5: 588;
    i32 rfp6: 23;
    i32 nmp1: 15;
    i32 nmp2: 154;
    i32 nmp3: 104;
    i32 nmp4: 189;
    i32 nmp5: 5;
    i32 nmp6: 248;
    i32 prob1: 265;
    i32 prob2: 60;
    i32 prob3: 4;
    i32 r1: 499;
    i32 r2: 434;
    i32 r3: 1263;
    i32 lmp1: 4;
    i32 lmp2: 17;
    i32 fp1: 107;
    i32 fp2: 48;
    i32 fp3: 90;
    i32 fp4: 85;
    i32 fp5: 9;
    i32 bnfp1: 118;
    i32 bnfp2: 80;
    i32 bnfp3: 91;
    i32 bnfp4: 67;
    i32 bnfp6: 6;
    i32 see1: 22;
    i32 see2: 31;
    i32 see3: 17;
    i32 see4: 104;
    i32 see5: 45;
    i32 see6: 46;
    i32 se2: 277;
    i32 se3: 67;
    i32 se4: 315;
    i32 se5: 16;
    i32 se6: 14;
    i32 lmr1: 523;
    i32 lmr2: 139;
    i32 lmr3: 477;
    i32 lmr4: 107;
    i32 lmr5: 46;
    i32 lmr6: 3689;
    i32 lmr7: 71;
    i32 lmr9: 454;
    i32 lmr10: 680;
    i32 lmr11: 817;
    i32 lmr13: 416;
    i32 lmr14: 549;
    i32 lmr15: 33;
    i32 lmr16: 1766;
    i32 lmr17: 980;
    i32 lmr18: 1026;
    i32 lmr19: 1639;
    i32 lmr20: 801;
    i32 lmr21: 1331;
    i32 lmr22: 15;
    i32 post1: 159;
    i32 post2: 62;
    i32 post3: 866;
    i32 post4: 38;
    i32 post5: 504;
    i32 fds1: 391;
    i32 fds2: 143;
    i32 fds3: 350;
    i32 fds4: 67;
    i32 fds5: 47;
    i32 fds6: 2806;
    i32 fds7: 51;
    i32 fds8: 776;
    i32 fds9: 536;
    i32 fds10: 1071;
    i32 fds11: 480;
    i32 fds12: 801;
    i32 fds13: 26;
    i32 fds14: 1391;
    i32 fds15: 1055;
    i32 fds16: 741;
    i32 fds17: 1477;
    i32 fds18: 854;
    i32 fds19: 1086;
    i32 fds20: 3072;
    i32 bonus1: 128;
    i32 bonus2: 54;
    i32 bonus3: 1136;
    i32 bonus4: 69;
    i32 malus1: 150;
    i32 malus2: 64;
    i32 malus3: 1459;
    i32 malus4: 24;
    i32 bonus5: 152;
    i32 bonus6: 69;
    i32 bonus7: 1624;
    i32 bonus8: 64;
    i32 malus5: 133;
    i32 malus6: 50;
    i32 malus7: 1122;
    i32 malus8: 36;
    i32 bonus9: 99;
    i32 bonus10: 56;
    i32 bonus11: 1265;
    i32 bonus12: 68;
    i32 malus10: 289;
    i32 malus11: 46;
    i32 malus12: 1032;
    i32 malus13: 30;
    i32 upd1: 82;
    i32 upd2: 53;
    i32 upd3: 845;
    i32 pcm1: 106;
    i32 pcm2: 148;
    i32 pcm3: 5;
    i32 pcm4: 237;
    i32 pcm5: 133;
    i32 pcm6: 291;
    i32 pcm7: 102;
    i32 pcm8: 151;
    i32 pcm9: 42;
    i32 pcm10: 1770;
    i32 pcm11: 158;
    i32 pcm12: 43;
    i32 pcm13: 1589;
    i32 qs1: 80;
    i32 qs2: 79;
    i32 corr1: 141;
    i32 corr2: 4038;
    i32 corr3: 3244;
    f32 red1: 977.5506;
    f32 red2: 443.8557;
    i32 qs_fb1: 32;
    i32 raise1: 17;
);
