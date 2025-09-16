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
    i32 eval1: 21269;
    i32 eval2: 1764;
    i32 eval3: 27455;
    i32 corr4: 82;
    i32 corr5: 104;
    i32 mp2: 119;
    i32 mp3: 2008;
    i32 mp4: 1057;
    i32 opt1: 119;
    i32 opt2: 237;
    i32 delta1: 24783;
    i32 hist1: 141;
    i32 hist2: 72;
    i32 hist3: 1539;
    i32 hist4: 68;
    i32 hist5: 99;
    i32 hist6: 61;
    i32 hist7: 1495;
    i32 hist8: 65;
    i32 hist9: 742;
    i32 hist10: 122;
    i32 hist11: 257;
    i32 hr1: 2427;
    i32 hr2: 975;
    i32 hr3: 63;
    i32 raz1: 316;
    i32 raz2: 237;
    i32 rfp1: 157;
    i32 rfp2: 31;
    i32 rfp3: 71;
    i32 rfp4: 23;
    i32 rfp5: 590;
    i32 rfp6: 23;
    i32 nmp1: 16;
    i32 nmp2: 156;
    i32 nmp3: 106;
    i32 nmp4: 212;
    i32 nmp6: 258;
    i32 prob1: 254;
    i32 prob2: 64;
    i32 r1: 490;
    i32 r2: 411;
    i32 r3: 1248;
    f32 lmp1: 3.638;
    f32 lmp2: 0.974;
    f32 lmp3: 1.893;
    f32 lmp4: 0.471;
    i32 lmp5: 17;
    i32 fp1: 106;
    i32 fp2: 49;
    i32 fp3: 93;
    i32 fp4: 83;
    i32 bnfp1: 121;
    i32 bnfp2: 73;
    i32 bnfp3: 93;
    i32 bnfp4: 70;
    i32 see1: 330;
    i32 see2: 31;
    i32 see3: 16;
    i32 see4: 101;
    i32 see5: 45;
    i32 see6: 46;
    i32 lmr1: 488;
    i32 lmr2: 139;
    i32 lmr3: 482;
    i32 lmr4: 108;
    i32 lmr5: 46;
    i32 lmr6: 3633;
    i32 lmr7: 69;
    i32 lmr9: 420;
    i32 lmr10: 677;
    i32 lmr11: 750;
    i32 lmr13: 390;
    i32 lmr14: 540;
    i32 lmr15: 30;
    i32 lmr16: 1701;
    i32 lmr17: 947;
    i32 lmr18: 1057;
    i32 lmr19: 1569;
    i32 lmr20: 799;
    i32 lmr21: 1377;
    i32 lmr22: 16;
    i32 post1: 158;
    i32 post2: 62;
    i32 post3: 850;
    i32 post4: 36;
    i32 post5: 493;
    i32 fds1: 376;
    i32 fds2: 153;
    i32 fds3: 352;
    i32 fds4: 68;
    i32 fds5: 46;
    i32 fds6: 2715;
    i32 fds7: 52;
    i32 fds8: 747;
    i32 fds9: 545;
    i32 fds10: 1070;
    i32 fds11: 492;
    i32 fds12: 792;
    i32 fds13: 26;
    i32 fds14: 1468;
    i32 fds15: 1049;
    i32 fds16: 746;
    i32 fds17: 1436;
    i32 fds18: 845;
    i32 fds19: 1067;
    i32 fds20: 3106;
    i32 bonus1: 124;
    i32 bonus2: 58;
    i32 bonus3: 1154;
    i32 bonus4: 70;
    i32 malus1: 153;
    i32 malus2: 64;
    i32 malus3: 1469;
    i32 malus4: 24;
    i32 bonus5: 152;
    i32 bonus6: 72;
    i32 bonus7: 1589;
    i32 bonus8: 64;
    i32 malus5: 133;
    i32 malus6: 51;
    i32 malus7: 1160;
    i32 malus8: 38;
    i32 bonus9: 101;
    i32 bonus10: 57;
    i32 bonus11: 1224;
    i32 bonus12: 64;
    i32 malus10: 305;
    i32 malus11: 46;
    i32 malus12: 1015;
    i32 malus13: 30;
    i32 upd1: 79;
    i32 upd2: 53;
    i32 upd3: 822;
    i32 pcm1: 104;
    i32 pcm2: 148;
    i32 pcm4: 219;
    i32 pcm5: 134;
    i32 pcm6: 302;
    i32 pcm7: 100;
    i32 pcm8: 154;
    i32 pcm9: 42;
    i32 pcm10: 1780;
    i32 pcm11: 151;
    i32 pcm12: 42;
    i32 pcm13: 1621;
    i32 qs1: 79;
    i32 corr1: 150;
    i32 corr2: 4197;
    i32 corr3: 3092;
    f32 red1: 975.753;
    f32 red2: 454.712;
    i32 qs_fb1: 32;
);
