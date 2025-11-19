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
    i32 delta1: 12;
    i32 delta2: 24616;
    i32 delta3: 30;
    i32 delta4: 64;
    i32 opt1: 119;
    i32 opt2: 237;
    i32 ttcut1: 141;
    i32 ttcut2: 72;
    i32 ttcut3: 1544;
    i32 ttcut4: 99;
    i32 ttcut5: 61;
    i32 ttcut6: 1509;
    i32 evalord1: 733;
    i32 evalord2: 123;
    i32 evalord3: 255;
    i32 hs1: 2397;
    i32 hs2: 963;
    i32 hs3: 63;
    i32 razor1: 320;
    i32 razor2: 237;
    i32 rfp1: 1280;
    i32 rfp2: 30;
    i32 rfp3: 75;
    i32 rfp4: 512;
    i32 rfp5: 32;
    i32 nmp1: 12;
    i32 nmp2: 158;
    i32 nmp3: 106;
    i32 nmp4: 233;
    i32 nmp5: 6308;
    i32 nmp6: 321;
    i32 probcut1: 259;
    i32 probcut2: 65;
    i32 se1: 277;
    i32 se2: 67;
    i32 se3: 315;
    i32 se4: 16;
    i32 red1: 489;
    i32 red2: 412;
    i32 red3: 1243;
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
    i32 see1: 2560;
    i32 see2: 31;
    i32 see3: 23;
    i32 see4: 102;
    i32 see5: 45;
    i32 see6: 46;
    i32 lmr1: 489;
    i32 lmr2: 137;
    i32 lmr3: 488;
    i32 lmr4: 109;
    i32 lmr5: 46;
    i32 lmr6: 3607;
    i32 lmr7: 69;
    i32 lmr8: 427;
    i32 lmr9: 677;
    i32 lmr10: 729;
    i32 lmr11: 393;
    i32 lmr12: 552;
    i32 lmr13: 1675;
    i32 lmr14: 934;
    i32 lmr15: 1049;
    i32 lmr16: 1555;
    i32 lmr17: 791;
    i32 lmr18: 1397;
    i32 dod1: 37;
    i32 dod2: 495;
    i32 dos1: 16;
    i32 fds1: 380;
    i32 fds2: 153;
    i32 fds3: 355;
    i32 fds4: 68;
    i32 fds5: 47;
    i32 fds6: 2667;
    i32 fds7: 52;
    i32 fds8: 750;
    i32 fds9: 1081;
    i32 fds10: 1478;
    i32 fds11: 1048;
    i32 fds12: 1438;
    i32 fds13: 1052;
    i32 fds14: 3034;
    i32 noisy1: 125;
    i32 noisy2: 57;
    i32 noisy3: 1175;
    i32 noisy4: 70;
    i32 noisy5: 153;
    i32 noisy6: 64;
    i32 noisy7: 1476;
    i32 noisy8: 24;
    i32 quiet1: 152;
    i32 quiet2: 73;
    i32 quiet3: 1569;
    i32 quiet4: 64;
    i32 quiet5: 133;
    i32 quiet6: 51;
    i32 quiet7: 1162;
    i32 quiet8: 37;
    i32 cont1: 102;
    i32 cont2: 56;
    i32 cont3: 1223;
    i32 cont4: 65;
    i32 cont5: 306;
    i32 cont6: 46;
    i32 cont7: 1018;
    i32 cont8: 30;
    i32 refut1: 78;
    i32 refut2: 52;
    i32 refut3: 811;
    i32 post1: 232;
    i32 post2: 94;
    i32 post3: 1400;
    i32 pcm1: 79;
    i32 pcm2: 147;
    i32 pcm3: 184;
    i32 pcm4: 128;
    i32 pcm5: 217;
    i32 pcm6: 132;
    i32 pcm7: 297;
    i32 pcm8: 100;
    i32 pcm9: 156;
    i32 pcm10: 42;
    i32 pcm11: 1789;
    i32 pcm12: 151;
    i32 pcm13: 41;
    i32 pcm14: 1630;
    i32 qs1: 79;
    i32 qs2: 32;
    i32 corrhist1: 150;
    i32 corrhist2: 4194;
    i32 corrhist3: 3164;
);
