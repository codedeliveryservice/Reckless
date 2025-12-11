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
    i32 delta1: 13;
    i32 delta2: 23682;
    i32 delta3: 26;
    i32 delta4: 61;
    i32 opt1: 158;
    i32 opt2: 191;
    i32 ttcut1: 190;
    i32 ttcut2: 80;
    i32 ttcut3: 1830;
    i32 ttcut4: 109;
    i32 ttcut5: 56;
    i32 ttcut6: 1337;
    i32 ttpcm1: 94;
    i32 ttpcm2: 183;
    i32 ttpcm3: 132;
    i32 ttpcm4: 240;
    i32 ttpcm5: 95;
    i32 ttpcm6: 163;
    i32 ttpcm7: 44;
    i32 ttpcm8: 2389;
    i32 ttpcm9: 122;
    i32 ttpcm10: 34;
    i32 ttpcm11: 1723;
    i32 evalord1: 865;
    i32 evalord2: 119;
    i32 evalord3: 325;
    i32 hs1: 2325;
    i32 hs2: 758;
    i32 hs3: 62;
    i32 razor1: 281;
    i32 razor2: 271;
    i32 rfp1: 1085;
    i32 rfp2: 25;
    i32 rfp3: 79;
    i32 rfp4: 500;
    i32 rfp5: 35;
    i32 nmp1: 10;
    i32 nmp2: 128;
    i32 nmp3: 133;
    i32 nmp4: 274;
    i32 nmp5: 6582;
    i32 nmp6: 273;
    i32 probcut1: 257;
    i32 probcut2: 75;
    i32 se1: 272;
    i32 se2: 57;
    i32 se3: 313;
    i32 se4: 14;
    i32 redbase1: 1209;
    i32 redbase2: 285;
    i32 red1: 443;
    i32 red2: 268;
    i32 red3: 1321;
    i32 red4: 425;
    i32 red5: 453;
    i32 lmp1: 19;
    i32 lmp2: 3219;
    i32 lmp3: 1093;
    i32 lmp4: 1252;
    i32 lmp5: 320;
    i32 fp1: 93;
    i32 fp2: 62;
    i32 fp3: 90;
    i32 fp4: 89;
    i32 bnfp1: 122;
    i32 bnfp2: 70;
    i32 bnfp3: 84;
    i32 bnfp4: 79;
    i32 see1: 1746;
    i32 see2: 33;
    i32 see3: 24;
    i32 see4: 86;
    i32 see5: 32;
    i32 see6: 42;
    i32 lmr1: 599;
    i32 lmr2: 152;
    i32 lmr3: 355;
    i32 lmr4: 102;
    i32 lmr5: 50;
    i32 lmr6: 3326;
    i32 lmr7: 68;
    i32 lmr8: 349;
    i32 lmr9: 714;
    i32 lmr10: 897;
    i32 lmr13: 1713;
    i32 lmr14: 1086;
    i32 lmr15: 884;
    i32 lmr16: 1498;
    i32 lmr17: 622;
    i32 lmr18: 1264;
    i32 lmr19: 907;
    i32 dod1: 43;
    i32 dod2: 482;
    i32 dos1: 14;
    i32 fds1: 406;
    i32 fds2: 154;
    i32 fds3: 235;
    i32 fds4: 65;
    i32 fds5: 47;
    i32 fds6: 2484;
    i32 fds7: 55;
    i32 fds8: 747;
    i32 fds9: 1080;
    i32 fds10: 1379;
    i32 fds11: 1211;
    i32 fds12: 1445;
    i32 fds13: 1166;
    i32 fds14: 3187;
    i32 fdsred: 5653;
    i32 noisy1: 111;
    i32 noisy2: 54;
    i32 noisy3: 861;
    i32 noisy4: 77;
    i32 noisy5: 173;
    i32 noisy6: 53;
    i32 noisy7: 1257;
    i32 noisy8: 23;
    i32 quiet1: 179;
    i32 quiet2: 75;
    i32 quiet3: 1335;
    i32 quiet4: 56;
    i32 quiet5: 156;
    i32 quiet6: 44;
    i32 quiet7: 1056;
    i32 quiet8: 41;
    i32 cont1: 115;
    i32 cont2: 67;
    i32 cont3: 972;
    i32 cont4: 50;
    i32 cont5: 343;
    i32 cont6: 47;
    i32 cont7: 856;
    i32 cont8: 21;
    i32 refut1: 86;
    i32 refut2: 58;
    i32 refut3: 778;
    i32 post1: 210;
    i32 post2: 87;
    i32 post3: 1663;
    i32 pcm1: 97;
    i32 pcm2: 159;
    i32 pcm3: 214;
    i32 pcm4: 112;
    i32 pcm5: 151;
    i32 pcm6: 95;
    i32 pcm7: 319;
    i32 pcm8: 113;
    i32 pcm9: 157;
    i32 pcm10: 33;
    i32 pcm11: 2564;
    i32 pcm12: 166;
    i32 pcm13: 37;
    i32 pcm14: 1268;
    i32 qs1: 109;
    i32 qs2: 42;
    i32 corrhist1: 140;
    i32 corrhist2: 5042;
    i32 corrhist3: 2895;
    i32 corrhist4: 88;
    i32 mp1: 43;
    i32 mp2: 108;
    i32 eval1: 21372;
    i32 eval2: 1536;
    i32 eval3: 27380;
    i32 shawnofthewalk: 42;
);
