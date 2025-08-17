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
    i32 delta1: 12;
    i32 delta2: 26802;
    i32 delta3: 40;
    i32 delta4: 15;
    i32 opt1: 112;
    i32 opt2: 235;
    i32 hist1: 134;
    i32 hist2: 72;
    i32 hist3: 1380;
    i32 hist4: 69;
    i32 hist5: 100;
    i32 hist6: 62;
    i32 hist7: 1415;
    i32 hist8: 69;
    i32 hist9: 709;
    i32 hist10: 59;
    i32 hist11: 138;
    i32 hr1: 2765;
    i32 hr2: 914;
    i32 hr3: 59;
    i32 se1: 5;
    i32 se2: 300;
    i32 se3: 64;
    i32 se4: 300;
    i32 se5: 16;
    i32 se6: 14;
    i32 raz1: 294;
    i32 raz2: 264;
    i32 rfp1: 10;
    i32 rfp2: 30;
    i32 rfp3: 70;
    i32 rfp4: 23;
    i32 rfp5: 559;
    i32 rfp6: 23;
    i32 nmp1: 15;
    i32 nmp2: 147;
    i32 nmp3: 105;
    i32 nmp4: 187;
    i32 nmp5: 5;
    i32 nmp6: 244;
    i32 prob1: 271;
    i32 prob2: 61;
    i32 r1: 494;
    i32 r2: 425;
    i32 r3: 1205;
    i32 lmp1: 4;
    i32 lmp2: 17;
    i32 fp1: 121;
    i32 fp2: 76;
    i32 fp3: 35;
    i32 fp4: 8;
    i32 bnfp1: 114;
    i32 bnfp2: 397;
    i32 bnfp3: 81;
    i32 bnfp4: 501;
    i32 bnfp5: 85;
    i32 see1: 22;
    i32 see2: 44;
    i32 see3: 19;
    i32 see4: 92;
    i32 see5: 45;
    i32 see6: 43;
    i32 see7: 13;
    i32 lmr1: 106;
    i32 lmr2: 574;
    i32 lmr3: 95;
    i32 lmr4: 557;
    i32 lmr5: 3268;
    i32 lmr6: 55;
    i32 lmr7: 303;
    i32 lmr8: 663;
    i32 lmr9: 652;
    i32 lmr10: 783;
    i32 lmr11: 796;
    i32 lmr12: 590;
    i32 lmr13: 573;
    i32 lmr14: 34;
    i32 lmr15: 1193;
    i32 lmr16: 794;
    i32 lmr17: 1232;
    i32 lmr18: 768;
    i32 lmr19: 1024;
    i32 lmr20: 40;
    f32 red1: 1000.0;
    f32 red2: 455.0;
    i32 post1: 162;
    i32 post2: 50;
    i32 post3: 1037;
    i32 bonus1: 128;
    i32 bonus2: 60;
    i32 bonus3: 1150;
    i32 bonus4: 69;
    i32 bonus5: 151;
    i32 bonus6: 68;
    i32 bonus7: 1597;
    i32 bonus8: 64;
    i32 bonus9: 97;
    i32 bonus10: 57;
    i32 bonus11: 1250;
    i32 bonus12: 69;
    i32 malus1: 145;
    i32 malus2: 67;
    i32 malus3: 1457;
    i32 malus4: 26;
    i32 malus5: 134;
    i32 malus6: 55;
    i32 malus7: 1273;
    i32 malus8: 34;
    i32 malus9: 200;
    i32 malus10: 277;
    i32 malus11: 49;
    i32 malus12: 978;
    i32 malus13: 28;
    i32 malus14: 126;
    i32 upd1: 80;
    i32 upd2: 55;
    i32 upd3: 800;
    i32 pcm1: 107;
    i32 pcm2: 141;
    i32 pcm3: 5;
    i32 pcm4: 231;
    i32 pcm5: 135;
    i32 pcm6: 289;
    i32 pcm7: 102;
    i32 pcm8: 148;
    i32 pcm9: 43;
    i32 pcm10: 1673;
    i32 pcm11: 148;
    i32 pcm12: 43;
    i32 pcm13: 1673;
    i32 qs1: 123;
    i32 qs2: 73;
    i32 corr1: 138;
    i32 corr2: 3964;
    i32 corr3: 3303;
    i32 corr4: 82;
    i32 corr5: 100;
    i32 eval1: 19768;
    i32 eval2: 1828;
    i32 eval3: 30145;
    i32 mp1: 37;
    i32 mp2: 110;
    i32 mp3: 2123;
    i32 mp4: 984;
);
