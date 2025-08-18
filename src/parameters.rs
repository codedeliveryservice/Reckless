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
	i32 delta2: 14911;
	i32 delta3: 39;
	i32 delta4: 13;
	i32 opt1: 126;
	i32 opt2: 124;
	i32 hist1: 147;
	i32 hist2: 65;
	i32 hist3: 1088;
	i32 hist4: 83;
	i32 hist5: 94;
	i32 hist6: 37;
	i32 hist7: 967;
	i32 hist8: 75;
	i32 hist9: 853;
	i32 hist10: 85;
	i32 hist11: 152;
	i32 hr1: 2985;
	i32 hr2: 770;
	i32 hr3: 55;
	i32 se1: 7;
	i32 se2: 138;
	i32 se3: 75;
	i32 se4: 232;
	i32 se5: 24;
	i32 se6: 7;
	i32 raz1: 179;
	i32 raz2: 267;
	i32 rfp1: 12;
	i32 rfp2: 49;
	i32 rfp3: 43;
	i32 rfp4: 18;
	i32 rfp5: 798;
	i32 rfp6: 27;
	i32 nmp1: 10;
	i32 nmp2: 192;
	i32 nmp3: 73;
	i32 nmp4: 214;
	i32 nmp5: 4;
	i32 nmp6: 242;
	i32 prob1: 340;
	i32 prob2: 56;
	i32 r1: 369;
	i32 r2: 435;
	i32 r3: 1073;
	i32 lmp1: 5;
	i32 lmp2: 11;
	i32 fp1: 96;
	i32 fp2: 46;
	i32 fp3: 39;
	i32 fp4: 5;
	i32 bnfp1: 293;
	i32 bnfp2: 395;
	i32 bnfp3: 47;
	i32 bnfp4: 697;
	i32 bnfp5: 103;
	i32 see1: 22;
	i32 see2: 12;
	i32 see3: 18;
	i32 see4: 113;
	i32 see5: 46;
	i32 see6: 30;
	i32 see7: 15;
	i32 lmr1: 120;
	i32 lmr2: 733;
	i32 lmr3: 120;
	i32 lmr4: 674;
	i32 lmr5: 2385;
	i32 lmr6: 73;
	i32 lmr7: 333;
	i32 lmr8: 700;
	i32 lmr9: 638;
	i32 lmr10: 950;
	i32 lmr11: 1185;
	i32 lmr12: 582;
	i32 lmr13: 787;
	i32 lmr14: 43;
	i32 lmr15: 829;
	i32 lmr16: 1145;
	i32 lmr17: 1380;
	i32 lmr18: 870;
	i32 lmr19: 1215;
	i32 lmr20: 43;
	f32 red1: 669.0;
	f32 red2: 314.0;
	i32 post1: 167;
	i32 post2: 42;
	i32 post3: 600;
	i32 bonus1: 123;
	i32 bonus2: 79;
	i32 bonus3: 1493;
	i32 bonus4: 40;
	i32 bonus5: 163;
	i32 bonus6: 114;
	i32 bonus7: 914;
	i32 bonus8: 60;
	i32 bonus9: 135;
	i32 bonus10: 68;
	i32 bonus11: 1390;
	i32 bonus12: 53;
	i32 malus1: 145;
	i32 malus2: 103;
	i32 malus3: 2654;
	i32 malus4: 28;
	i32 malus5: 179;
	i32 malus6: 44;
	i32 malus7: 1392;
	i32 malus8: 28;
	i32 malus9: 154;
	i32 malus10: 478;
	i32 malus11: 59;
	i32 malus12: 964;
	i32 malus13: 28;
	i32 malus14: 97;
	i32 upd1: 49;
	i32 upd2: 45;
	i32 upd3: 1142;
	i32 pcm1: 162;
	i32 pcm2: 159;
	i32 pcm3: 4;
	i32 pcm4: 137;
	i32 pcm5: 163;
	i32 pcm6: 146;
	i32 pcm7: 75;
	i32 pcm8: 168;
	i32 pcm9: 41;
	i32 pcm10: 1023;
	i32 pcm11: 199;
	i32 pcm12: 60;
	i32 pcm13: 3236;
	i32 qs1: 135;
	i32 qs2: 67;
	i32 corr1: 173;
	i32 corr2: 2594;
	i32 corr3: 2477;
	i32 corr4: 88;
	i32 corr5: 121;
	i32 eval1: 26877;
	i32 eval2: 1943;
	i32 eval3: 23886;
	i32 mp1: 23;
	i32 mp2: 114;
	i32 mp3: 2365;
	i32 mp4: 1030;
);
