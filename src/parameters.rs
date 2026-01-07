// for the shawnofthewalk parameter
#![allow(dead_code)]

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
    i32 eval1: 21372;
    i32 eval2: 1536;
    i32 eval3: 27380;
    i32 mp1: 43;
    i32 mp2: 108;
    i32 mp3: 16;
    i32 mp4: 1024;
    i32 mp5: 1024;
    i32 mp6: 1024;
    i32 mp7: 1024;
    i32 mp8: 1024;
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
    i32 ttcut7: 4;
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
    i32 ttpcm12: 8;
    i32 ttpcm13: 5;
    i32 evalord1: 865;
    i32 evalord2: 119;
    i32 evalord3: 325;
    i32 hs1: 2325;
    i32 hs2: 0;
    i32 hs3: 758;
    i32 hs4: 62;
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
    i32 nmp5: 5140;
    i32 nmp6: 273;
    i32 nmp7: 512;
    i32 nmp8: 1024;
    i32 probcut1: 257;
    i32 probcut2: 75;
    i32 probcut3: 50;
    i32 probcut4: 300;
    i32 probcut5: 300;
    i32 lmp1: 19;
    i32 lmp2: 3219;
    i32 lmp3: 1093;
    i32 lmp4: 1252;
    i32 lmp5: 320;
    i32 fp1: 94;
    i32 fp2: 61;
    i32 fp3: 87;
    i32 fp4: 116;
    i32 fp5: 14;
    i32 bnfp1: 68;
    i32 bnfp2: 68;
    i32 bnfp3: 83;
    i32 bnfp4: 24;
    i32 bnfp5: 12;
    i32 see1: 16;
    i32 see2: 50;
    i32 see3: 21;
    i32 see4: 25;
    i32 see5: 8;
    i32 see6: 36;
    i32 see7: 33;
    i32 see8: 10;
    i32 lmr1: 240;
    i32 lmr2: 28;
    i32 lmr3: 28;
    i32 lmr4: 68;
    i32 lmr5: 3326;
    i32 lmr6: 2031;
    i32 lmr7: 152;
    i32 lmr8: 1563;
    i32 lmr9: 102;
    i32 lmr10: 50;
    i32 lmr11: 425;
    i32 lmr12: 453;
    i32 lmr13: 349;
    i32 lmr14: 714;
    i32 lmr15: 897;
    i32 lmr16: 907;
    i32 lmr17: 1713;
    i32 lmr18: 1086;
    i32 lmr19: 443;
    i32 lmr20: 268;
    i32 lmr21: 1321;
    i32 lmr22: 884;
    i32 lmr23: 1498;
    i32 lmr24: 622;
    i32 lmr25: 1264;
    i32 dod1: 43;
    i32 dod2: 482;
    i32 dos1: 14;
    i32 fds1: 246;
    i32 fds2: 25;
    i32 fds3: 25;
    i32 fds4: 55;
    i32 fds5: 2484;
    i32 fds6: 1634;
    i32 fds7: 154;
    i32 fds8: 1423;
    i32 fds9: 65;
    i32 fds10: 47;
    i32 fds11: 747;
    i32 fds12: 1080;
    i32 fds13: 1379;
    i32 fds14: 1211;
    i32 fds15: 443;
    i32 fds16: 268;
    i32 fds17: 1321;
    i32 fds18: 1445;
    i32 fds19: 1166;
    i32 fds20: 3187;
    i32 fdsred1: 5653;
    i32 pvs1: 8;
    i32 alpharaise1: 17;
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
    i32 pcm15: 57;
    i32 pcm16: 5;
    i32 pcm17: 8;
    i32 qs1: 109;
    i32 qs2: 42;
    i32 qs3: 79;
    i32 corrhist1: 1024;
    i32 corrhist2: 1024;
    i32 corrhist3: 1024;
    i32 corrhist4: 1024;
    i32 corrhist5: 1024;
    i32 corrhist6: 88;
    i32 corrhist7: 140;
    i32 corrhist8: 5042;
    i32 corrhist9: 2895;
    i32 quiettable1: 1940;
    i32 quiettable2: 6029;
    i32 noisytable1: 4449;
    i32 noisytable2: 8148;
    i32 corrtable1: 14734;
    i32 corrtable2: 16222;
    i32 conttable1: 15324;
    i32 shawnofthewalk: 420;
);
