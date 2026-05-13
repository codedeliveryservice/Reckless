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
    i32 eval1: 21454;
    i32 eval2: 1543;
    i32 eval3: 26663;
    i32 delta1: 18;
    i32 delta2: 25704;
    i32 delta3: 26;
    i32 delta4: 60;
    i32 optimism1: 157;
    i32 optimism2: 173;
    i32 ttcut1: 177;
    i32 ttcut2: 73;
    i32 ttcut3: 1702;
    i32 ttcut4: 105;
    i32 ttcut5: 69;
    i32 ttcut6: 1169;
    i32 evalord1: 880;
    i32 evalord2: 133;
    i32 evalord3: 361;
    i32 evalord4: 7;
    i32 hs1: 2379;
    i32 hs2: 52;
    i32 razor1: 265;
    i32 razor2: 267;
    i32 rfp1: 1189;
    i32 rfp2: 96;
    i32 rfp3: 23;
    i32 rfp4: 600;
    i32 rfp5: 60;
    i32 rfp6: 19;
    i32 rfp7: 1;
    i32 nmp1: 9;
    i32 nmp2: 108;
    i32 nmp3: 96;
    i32 nmp4: 18;
    i32 nmp5: 320;
    i32 nmp6: 2;
    i32 nmp7: 624;
    i32 nmp8: 4311;
    i32 nmp9: 1024;
    i32 nmp10: 260;
    i32 nmp11: 493;
    i32 nmp12: 1003;
    i32 probcut1: 282;
    i32 probcut2: 80;
    i32 probcut3: 305;
    i32 probcut4: 256;
    i32 se1: 216;
    i32 se2: 48;
    i32 se3: 15;
    i32 se4: 19;
    i32 se5: 263;
    i32 se6: 55;
    i32 se7: 17;
    i32 se8: 13;
    i32 se9: 33;
    i32 lmp1: 2697;
    i32 lmp2: 77;
    i32 lmp3: 1510;
    i32 lmp4: 70;
    i32 fp1: 79;
    i32 fp2: 63;
    i32 fp3: 83;
    i32 fp4: 542;
    i32 fp5: 135;
    i32 fp6: 16;
    i32 bnfp1: 80;
    i32 bnfp2: 71;
    i32 bnfp3: 24;
    i32 bnfp4: 11;
    i32 hp1: 5;
    i32 hp2: 1024;
    i32 see1: 15;
    i32 see2: 52;
    i32 see3: 23;
    i32 see4: 25;
    i32 see5: 7;
    i32 see6: 31;
    i32 see7: 32;
    i32 see8: 16;
    i32 lmr1: 256;
    i32 lmr3: 3403;
    i32 lmr4: 1405;
    i32 lmr5: 459;
    i32 lmr6: 286;
    i32 lmr7: 1971;
    i32 lmr8: 179;
    i32 lmr9: 1424;
    i32 lmr10: 107;
    i32 lmr11: 463;
    i32 lmr12: 426;
    i32 lmr13: 368;
    i32 lmr14: 570;
    i32 lmr15: 722;
    i32 lmr16: 1810;
    i32 lmr17: 2113;
    i32 lmr18: 414;
    i32 lmr19: 238;
    i32 lmr20: 1014;
    i32 lmr21: 939;
    i32 lmr22: 992;
    i32 lmr23: 567;
    i32 lmr24: 162;
    i32 lmr25: 2045;
    i32 lmr26: 462;
    i32 lmr27: 126;
    u64 lmr28: 25;
    i32 lmr29: 63;
    i32 lmr30: 384;
    i32 lmr31: 384;
    i32 lmr32: 64;
    i32 lmr33: 96;
    i32 fds1: 243;
    i32 fds3: 2382;
    i32 fds4: 1385;
    i32 fds5: 136;
    i32 fds6: 1049;
    i32 fds7: 55;
    i32 fds8: 924;
    i32 fds9: 1075;
    i32 fds10: 1366;
    i32 fds11: 2045;
    i32 fds12: 402;
    i32 fds13: 232;
    i32 fds14: 1426;
    i32 fds15: 1454;
    i32 fds16: 326;
    i32 fds17: 163;
    i32 fds18: 1943;
    i32 fds19: 3192;
    i32 fds20: 577;
    i32 fds21: 123;
    u64 fds22: 24;
    i32 fds23: 58;
    i32 fds24: 2757;
    i32 fds25: 5670;
    i32 fds26: 256;
    i32 dod1: 54;
    i32 dos1: 8;
    i32 noisy1: 89;
    i32 noisy2: 748;
    i32 noisy3: 45;
    i32 noisy4: 74;
    i32 noisy5: 179;
    i32 noisy6: 1391;
    i32 noisy7: 57;
    i32 noisy8: 23;
    i32 quiet1: 185;
    i32 quiet2: 1648;
    i32 quiet3: 85;
    i32 quiet4: 58;
    i32 quiet5: 162;
    i32 quiet6: 1198;
    i32 quiet7: 46;
    i32 quiet8: 34;
    i32 cont1: 107;
    i32 cont2: 1051;
    i32 cont3: 64;
    i32 cont4: 45;
    i32 cont5: 399;
    i32 cont6: 933;
    i32 cont7: 53;
    i32 cont8: 17;
    i32 refut1: 89;
    i32 refut2: 57;
    i32 refut3: 807;
    i32 post1: 196;
    i32 post2: 87;
    i32 post3: 1696;
    i32 pcm11: 148;
    i32 pcm12: 39;
    i32 pcm13: 1194;
    i32 pcm14: 59;
    i32 pcm15: 526;
    i32 qs1: 8;
    i32 qs2: 71;
    i32 qs3: 77;
    i32 qs4: 94;
    i32 corrhist1: 69;
    i32 corrhist2: 146;
    i32 corrhist3: 4449;
    i32 corrhist4: 2659;
    i32 mp1: 1973;
    i32 mp2: 1573;
    i32 mp3: 956;
    i32 mp4: 987;
    i32 mp5: 944;
    i32 mp6: 9503;
    i32 mp7: 8074;
    i32 mp8: 5182;
    i32 mp9: 4255;
    i32 mp10: 8297;
    i32 mp11: 8292;
    i32 mp12: 13144;
    i32 mp13: 21081;
    i32 mp14: 15704;
    i32 mp15: 4057;
    i32 mp16: 39;
    i32 mp17: 107;
    f32 lerp1: 0.63;
    f32 lerp2: 0.24;
    f32 lerp3: 0.34;
    f32 lerp4: 0.12;
    f32 lerp5: 0.69;
    f32 lerp6: 0.45;
);
