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
    i32 delta1: 14;
    i32 delta2: 13809;
    i32 delta3: 48;
    i32 delta4: 14;
    i32 opt1: 97;
    i32 opt2: 167;
    i32 hist1: 181;
    i32 hist2: 60;
    i32 hist3: 1153;
    i32 hist4: 53;
    i32 hist5: 89;
    i32 hist6: 53;
    i32 hist7: 1193;
    i32 hist8: 69;
    i32 hist9: 780;
    i32 hist10: 80;
    i32 hist11: 163;
    i32 hr1: 3909;
    i32 hr2: 775;
    i32 hr3: 41;
    i32 se1: 6;
    i32 se2: 252;
    i32 se3: 65;
    i32 se4: 197;
    i32 se5: 17;
    i32 se6: 13;
    i32 raz1: 219;
    i32 raz2: 259;
    i32 rfp1: 10;
    i32 rfp2: 33;
    i32 rfp3: 56;
    i32 rfp4: 20;
    i32 rfp5: 608;
    i32 rfp6: 24;
    i32 nmp1: 13;
    i32 nmp2: 141;
    i32 nmp3: 106;
    i32 nmp4: 232;
    i32 nmp5: 5;
    i32 nmp6: 319;
    i32 prob1: 222;
    i32 prob2: 48;
    i32 r1: 492;
    i32 r2: 537;
    i32 r3: 1611;
    i32 lmp1: 4;
    i32 lmp2: 19;
    i32 fp1: 103;
    i32 fp2: 68;
    i32 fp3: 42;
    i32 fp4: 7;
    i32 bnfp1: 170;
    i32 bnfp2: 454;
    i32 bnfp3: 69;
    i32 bnfp4: 522;
    i32 bnfp5: 72;
    i32 see1: 21;
    i32 see2: 27;
    i32 see3: 21;
    i32 see4: 104;
    i32 see5: 53;
    i32 see6: 37;
    i32 see7: 13;
    i32 lmr1: 68;
    i32 lmr2: 655;
    i32 lmr3: 95;
    i32 lmr4: 585;
    i32 lmr5: 3017;
    i32 lmr6: 54;
    i32 lmr7: 387;
    i32 lmr8: 734;
    i32 lmr9: 640;
    i32 lmr10: 817;
    i32 lmr11: 897;
    i32 lmr12: 668;
    i32 lmr13: 534;
    i32 lmr14: 39;
    i32 lmr15: 1486;
    i32 lmr16: 892;
    i32 lmr17: 1432;
    i32 lmr18: 844;
    i32 lmr19: 838;
    i32 lmr20: 43;
    f32 red1: 907.2985014521523;
    f32 red2: 461.4855476025686;
    i32 post1: 168;
    i32 post2: 49;
    i32 post3: 571;
    i32 bonus1: 170;
    i32 bonus2: 67;
    i32 bonus3: 1357;
    i32 bonus4: 35;
    i32 bonus5: 170;
    i32 bonus6: 88;
    i32 bonus7: 1622;
    i32 bonus8: 55;
    i32 bonus9: 104;
    i32 bonus10: 66;
    i32 bonus11: 1041;
    i32 bonus12: 65;
    i32 malus1: 143;
    i32 malus2: 109;
    i32 malus3: 1713;
    i32 malus4: 29;
    i32 malus5: 136;
    i32 malus6: 49;
    i32 malus7: 1339;
    i32 malus8: 27;
    i32 malus9: 182;
    i32 malus10: 376;
    i32 malus11: 55;
    i32 malus12: 1028;
    i32 malus13: 21;
    i32 malus14: 128;
    i32 upd1: 60;
    i32 upd2: 47;
    i32 upd3: 1003;
    i32 pcm1: 127;
    i32 pcm2: 152;
    i32 pcm3: 5;
    i32 pcm4: 227;
    i32 pcm5: 122;
    i32 pcm6: 255;
    i32 pcm7: 59;
    i32 pcm8: 148;
    i32 pcm9: 35;
    i32 pcm10: 1604;
    i32 pcm11: 182;
    i32 pcm12: 50;
    i32 pcm13: 2216;
    i32 qs1: 109;
    i32 qs2: 67;
    i32 corr1: 106;
    i32 corr2: 2964;
    i32 corr3: 2422;
    i32 corr4: 97;
    i32 corr5: 105;
    i32 eval1: 27198;
    i32 eval2: 2236;
    i32 eval3: 26155;
    i32 mp1: 35;
    i32 mp2: 99;
    i32 mp3: 2003;
    i32 mp4: 976;
);
