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
    i32 delta2: 26411;
    i32 delta3: 44;
    i32 delta4: 14;
    i32 optimism1: 114;
    i32 optimism2: 240;
    i32 tt_cut1: 5;
    i32 tt_cut2: 137;
    i32 tt_cut3: 73;
    i32 tt_cut4: 1405;
    i32 tt_cut5: 105;
    i32 tt_cut6: 63;
    i32 tt_cut7: 1435;
    i32 static1: 674;
    i32 static2: 61;
    i32 static3: 144;
    i32 hs1: 2691;
    i32 hs2: 905;
    i32 hs3: 69;
    i32 razor1: 303;
    i32 razor2: 260;
    i32 rfp1: 7;
    i32 rfp2: 80;
    i32 rfp3: 72;
    i32 rfp4: 25;
    i32 rfp5: 556;
    i32 rfp6: 24;
    i32 nmp1: 15;
    i32 nmp2: 159;
    i32 nmp3: 102;
    i32 nmp4: 185;
    i32 nmp5: 225;
    i32 probcut1: 280;
    i32 probcut2: 63;
    i32 r1: 500;
    i32 r2: 416;
    i32 r3: 1200;
    i32 r4: 137;
    i32 lmp1: 18;
    i32 fp1: 122;
    i32 fp2: 78;
    i32 fp3: 32;
    i32 fp4: 9;
    i32 bnfp1: 111;
    i32 bnfp2: 396;
    i32 bnfp3: 80;
    i32 bnfp4: 500;
    i32 bnfp5: 85;
    i32 bnfp6: 6;
    i32 see1: 24;
    i32 see2: 43;
    i32 see3: 0;
    i32 see4: 94;
    i32 see5: 48;
    i32 see6: 42;
    i32 see7: 0;
    i32 se1: 64;
    i32 se2: 14;
    i32 lmr1: 98;
    i32 lmr2: 568;
    i32 lmr3: 98;
    i32 lmr4: 568;
    i32 lmr5: 3295;
    i32 lmr6: 54;
    i32 lmr7: 295;
    i32 lmr8: 683;
    i32 lmr9: 647;
    i32 lmr10: 791;
    i32 lmr11: 768;
    i32 lmr12: 614;
    i32 lmr13: 576;
    i32 lmr14: 34;
    i32 lmr15: 1141;
    i32 lmr16: 820;
    i32 lmr17: 1196;
    i32 dod1: 46;
    i32 dod2: 512;
    i32 post1: 152;
    i32 post2: 50;
    i32 post3: 973;
    i32 raise1: 15;
    i32 hist1: 124;
    i32 hist2: 65;
    i32 hist3: 1177;
    i32 hist4: 145;
    i32 hist5: 75;
    i32 hist6: 1403;
    i32 hist7: 14;
    i32 hist8: 148;
    i32 hist9: 71;
    i32 hist10: 1458;
    i32 hist11: 64;
    i32 hist12: 125;
    i32 hist13: 52;
    i32 hist14: 1263;
    i32 hist15: 17;
    i32 hist16: 196;
    i32 hist17: 114;
    i32 hist18: 53;
    i32 hist19: 1318;
    i32 hist20: 64;
    i32 hist21: 244;
    i32 hist22: 51;
    i32 hist23: 907;
    i32 hist24: 15;
    i32 hist25: 128;
    i32 pcm1: 102;
    i32 pcm2: 141;
    i32 pcm3: 227;
    i32 pcm4: 129;
    i32 pcm5: 277;
    i32 pcm6: 101;
    i32 pcm7: 137;
    i32 pcm8: 43;
    i32 pcm9: 1563;
    i32 qs1: 129;
    i32 qs2: 75;
    i32 corr1: 128;
    i32 corr2: 3927;
    i32 corr3: 3373;
    i32 corr4: 108;
    i32 corr5: 108;
    i32 eval1: 21682;
    i32 eval2: 1923;
    i32 eval3: 28993;
    i32 mp1: 34;
    i32 mp2: 107;
    i32 mp3: 2238;
    i32 mp4: 909;
    i32 max1: 2048;
    i32 max2: 6144;
    i32 max3: 4096;
    i32 max4: 8192;
    i32 max5: 16384;
    i32 max6: 16384;
    i32 max7: 16384;
);
