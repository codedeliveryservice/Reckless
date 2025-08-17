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
    i32 delta1: 16;
    i32 delta2: 13649;
    i32 delta3: 53;
    i32 delta4: 12;
    i32 opt1: 106;
    i32 opt2: 174;
    i32 hist1: 163;
    i32 hist2: 68;
    i32 hist3: 947;
    i32 hist4: 58;
    i32 hist5: 104;
    i32 hist6: 52;
    i32 hist7: 1169;
    i32 hist8: 69;
    i32 hist9: 715;
    i32 hist10: 84;
    i32 hist11: 138;
    i32 hr1: 3707;
    i32 hr2: 898;
    i32 hr3: 48;
    i32 se1: 6;
    i32 se2: 273;
    i32 se3: 69;
    i32 se4: 176;
    i32 se5: 18;
    i32 se6: 8;
    i32 raz1: 205;
    i32 raz2: 271;
    i32 rfp1: 10;
    i32 rfp2: 31;
    i32 rfp3: 62;
    i32 rfp4: 16;
    i32 rfp5: 644;
    i32 rfp6: 22;
    i32 nmp1: 13;
    i32 nmp2: 144;
    i32 nmp3: 100;
    i32 nmp4: 188;
    i32 nmp5: 6;
    i32 nmp6: 317;
    i32 prob1: 227;
    i32 prob2: 54;
    i32 r1: 522;
    i32 r2: 566;
    i32 r3: 1401;
    i32 lmp1: 4;
    i32 lmp2: 19;
    i32 fp1: 94;
    i32 fp2: 73;
    i32 fp3: 40;
    i32 fp4: 6;
    i32 bnfp1: 190;
    i32 bnfp2: 444;
    i32 bnfp3: 51;
    i32 bnfp4: 479;
    i32 bnfp5: 77;
    i32 see1: 22;
    i32 see2: 22;
    i32 see3: 23;
    i32 see4: 109;
    i32 see5: 57;
    i32 see6: 38;
    i32 see7: 14;
    i32 lmr1: 87;
    i32 lmr2: 659;
    i32 lmr3: 83;
    i32 lmr4: 575;
    i32 lmr5: 2866;
    i32 lmr6: 61;
    i32 lmr7: 406;
    i32 lmr8: 763;
    i32 lmr9: 663;
    i32 lmr10: 848;
    i32 lmr11: 888;
    i32 lmr12: 716;
    i32 lmr13: 560;
    i32 lmr14: 40;
    i32 lmr15: 1652;
    i32 lmr16: 983;
    i32 lmr17: 1363;
    i32 lmr18: 967;
    i32 lmr19: 929;
    i32 lmr20: 43;
    f32 red1: 872.0;
    f32 red2: 441.0;
    i32 post1: 162;
    i32 post2: 47;
    i32 post3: 651;
    i32 bonus1: 178;
    i32 bonus2: 60;
    i32 bonus3: 1209;
    i32 bonus4: 39;
    i32 bonus5: 201;
    i32 bonus6: 81;
    i32 bonus7: 1571;
    i32 bonus8: 48;
    i32 bonus9: 108;
    i32 bonus10: 64;
    i32 bonus11: 1068;
    i32 bonus12: 59;
    i32 malus1: 148;
    i32 malus2: 113;
    i32 malus3: 2025;
    i32 malus4: 25;
    i32 malus5: 132;
    i32 malus6: 49;
    i32 malus7: 1381;
    i32 malus8: 26;
    i32 malus9: 179;
    i32 malus10: 420;
    i32 malus11: 54;
    i32 malus12: 954;
    i32 malus13: 20;
    i32 malus14: 118;
    i32 upd1: 71;
    i32 upd2: 41;
    i32 upd3: 992;
    i32 pcm1: 149;
    i32 pcm2: 166;
    i32 pcm3: 5;
    i32 pcm4: 250;
    i32 pcm5: 146;
    i32 pcm6: 260;
    i32 pcm7: 64;
    i32 pcm8: 148;
    i32 pcm9: 46;
    i32 pcm10: 1601;
    i32 pcm11: 164;
    i32 pcm12: 58;
    i32 pcm13: 2160;
    i32 qs1: 104;
    i32 qs2: 65;
    i32 corr1: 118;
    i32 corr2: 2372;
    i32 corr3: 2577;
    i32 corr4: 99;
    i32 corr5: 123;
    i32 eval1: 26911;
    i32 eval2: 2226;
    i32 eval3: 26851;
    i32 mp1: 31;
    i32 mp2: 106;
    i32 mp3: 2152;
    i32 mp4: 1127;
);
