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
    i32 eval1: 20664;
    i32 eval2: 1487;
    i32 eval3: 26685;
    i32 delta1: 15;
    i32 delta2: 25833;
    i32 delta3: 28;
    i32 delta4: 62;
    i32 optimism1: 159;
    i32 optimism2: 186;
    i32 ttcut1: 175;
    i32 ttcut2: 79;
    i32 ttcut3: 1637;
    i32 ttcut4: 114;
    i32 ttcut5: 57;
    i32 ttcut6: 1284;
    i32 evalord1: 824;
    i32 evalord2: 133;
    i32 evalord3: 348;
    i32 hs1: 2367;
    i32 hs2: 59;
    i32 razor1: 295;
    i32 razor2: 261;
    i32 rfp1: 1165;
    i32 rfp2: 80;
    i32 rfp3: 25;
    i32 rfp4: 560;
    i32 rfp5: 59;
    i32 rfp6: 30;
    i32 rfp7: 0;
    i32 nmp1: 8;
    i32 nmp2: 116;
    i32 nmp3: 106;
    i32 nmp4: 20;
    i32 nmp5: 304;
    i32 nmp6: 0;
    i32 nmp7: 600;
    i32 nmp8: 5335;
    i32 nmp9: 260;
    i32 nmp10: 493;
    i32 nmp11: 1003;
    i32 probcut1: 270;
    i32 probcut2: 75;
    i32 probcut3: 319;
    i32 probcut4: 260;
    i32 se1: 196;
    i32 se2: 58;
    i32 se3: 16;
    i32 se4: 16;
    i32 se5: 249;
    i32 se6: 58;
    i32 se7: 16;
    i32 se8: 15;
    i32 se9: 32;
    i32 lmp1: 3006;
    i32 lmp2: 70;
    i32 lmp3: 1455;
    i32 lmp4: 68;
    i32 fp1: 79;
    i32 fp2: 64;
    i32 fp3: 84;
    i32 fp4: 560;
    i32 fp5: 146;
    i32 fp6: 15;
    i32 bnfp1: 71;
    i32 bnfp2: 68;
    i32 bnfp3: 23;
    i32 bnfp4: 11;
    i32 see1: 17;
    i32 see2: 52;
    i32 see3: 21;
    i32 see4: 20;
    i32 see5: 8;
    i32 see6: 36;
    i32 see7: 32;
    i32 see8: 11;
    i32 lmr1: 225;
    i32 lmr2: 68;
    i32 lmr3: 3297;
    i32 lmr4: 1306;
    i32 lmr5: 546;
    i32 lmr6: 322;
    i32 lmr7: 1806;
    i32 lmr8: 166;
    i32 lmr9: 1449;
    i32 lmr10: 109;
    i32 lmr11: 424;
    i32 lmr12: 433;
    i32 lmr13: 361;
    i32 lmr14: 636;
    i32 lmr15: 830;
    i32 lmr16: 1818;
    i32 lmr17: 2118;
    i32 lmr18: 430;
    i32 lmr19: 263;
    i32 lmr20: 1096;
    i32 lmr21: 1021;
    i32 lmr22: 1515;
    i32 lmr23: 512;
    i32 lmr24: 160;
    i32 lmr25: 2048;
    i32 lmr26: 485;
    i32 lmr27: 129;
    u64 lmr28: 23;
    i32 lmr29: 64;
    i32 fds1: 232;
    i32 fds2: 48;
    i32 fds3: 2408;
    i32 fds4: 1429;
    i32 fds5: 152;
    i32 fds6: 1053;
    i32 fds7: 67;
    i32 fds8: 936;
    i32 fds9: 1080;
    i32 fds10: 1543;
    i32 fds11: 2058;
    i32 fds12: 409;
    i32 fds13: 254;
    i32 fds14: 1488;
    i32 fds15: 1360;
    i32 fds16: 400;
    i32 fds17: 160;
    i32 fds18: 2048;
    i32 fds19: 3281;
    i32 fds20: 562;
    i32 fds21: 130;
    u64 fds22: 23;
    i32 fds23: 64;
    i32 fds24: 2864;
    i32 fds25: 5585;
    i32 dod1: 61;
    i32 dos1: 5;
    i32 pvs1: 8;
    i32 noisy1: 115;
    i32 noisy2: 778;
    i32 noisy3: 50;
    i32 noisy4: 77;
    i32 noisy5: 176;
    i32 noisy6: 1343;
    i32 noisy7: 51;
    i32 noisy8: 21;
    i32 quiet1: 172;
    i32 quiet2: 1508;
    i32 quiet3: 76;
    i32 quiet4: 55;
    i32 quiet5: 156;
    i32 quiet6: 1065;
    i32 quiet7: 45;
    i32 quiet8: 36;
    i32 cont1: 99;
    i32 cont2: 995;
    i32 cont3: 65;
    i32 cont4: 49;
    i32 cont5: 371;
    i32 cont6: 914;
    i32 cont7: 44;
    i32 cont8: 18;
    i32 refut1: 90;
    i32 refut2: 58;
    i32 refut3: 789;
    i32 post1: 194;
    i32 post2: 89;
    i32 post3: 1595;
    i32 pcm1: 78;
    i32 pcm2: 16;
    i32 pcm3: 116;
    i32 pcm4: 138;
    i32 pcm5: 93;
    i32 pcm6: 321;
    i32 pcm7: 128;
    i32 pcm8: 165;
    i32 pcm9: 35;
    i32 pcm10: 2467;
    i32 pcm11: 159;
    i32 pcm12: 39;
    i32 pcm13: 1160;
    i32 pcm14: 60;
    i32 pcm15: 600;
    i32 pcm16: 224;
    i32 qs1: 8;
    i32 qs2: 64;
    i32 qs3: 79;
    i32 qs4: 106;
    i32 corrhist1: 73;
    i32 corrhist2: 142;
    i32 corrhist3: 4771;
    i32 corrhist4: 3001;
    i32 mp1: 2048;
    i32 mp2: 1536;
    i32 mp3: 1024;
    i32 mp4: 1024;
    i32 mp5: 1024;
    i32 mp6: 9325;
    i32 mp7: 7584;
    i32 mp8: 5000;
    i32 mp9: 4000;
    i32 mp10: 7768;
    i32 mp11: 8218;
    i32 mp12: 13424;
    i32 mp13: 20208;
    i32 mp14: 16384;
    i32 mp15: 4000;
    i32 mp16: 45;
    i32 mp17: 111;
    i32 history1: 1852;
    i32 history2: 6324;
    i32 history3: 4524;
    i32 history4: 7826;
    i32 history5: 16282;
    i32 history6: 15168;
    f32 lerp1: 0.66;
    f32 lerp2: 0.25;
    f32 lerp3: 0.34;
    f32 lerp4: 0.16;
    f32 lerp5: 0.66;
    f32 lerp6: 0.5;
);
