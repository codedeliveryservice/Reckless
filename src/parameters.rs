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
    i32 eval1: 21032;
    i32 eval2: 1548;
    i32 eval3: 27015;
    i32 delta1: 23;
    i32 delta2: 26394;
    i32 delta3: 26;
    i32 delta4: 60;
    i32 delta5: 7;
    i32 evalstability1: 12;
    i32 optimism1: 113;
    i32 optimism2: 201;
    f32 tm1: 3.1838;
    f32 tm2: 2.6554;
    f32 tm3: 0.5460;
    f32 tm4: 0.7426;
    f32 tm5: 0.0480;
    f32 tm6: 0.7214;
    f32 tm7: 1.4031;
    f32 tm8: 1.2881;
    f32 tm9: 0.0440;
    f32 tm10: 0.7160;
    f32 tm11: 1.2664;
    f32 tm12: 0.0416;
    f32 tm13: 0.8642;
    f32 tm14: 1.1500;
    f32 tm15: 0.2526;
    i32 ttcut1: 190;
    i32 ttcut2: 81;
    i32 ttcut3: 1691;
    i32 ttcut4: 96;
    i32 ttcut5: 73;
    i32 ttcut6: 1206;
    i32 evalord1: 812;
    i32 evalord2: 144;
    i32 evalord3: 324;
    i32 hs1: 2249;
    i32 hs2: 57;
    i32 razor1: 237;
    i32 razor2: 254;
    i32 rfp1: 1140;
    i32 rfp2: 120;
    i32 rfp3: 22;
    i32 rfp4: 669;
    i32 rfp5: 54;
    i32 rfp6: -19;
    i32 rfp7: 2;
    i32 nmp1: 9;
    i32 nmp2: 110;
    i32 nmp3: 94;
    i32 nmp4: 21;
    i32 nmp5: 337;
    i32 nmp6: 2;
    i32 nmp7: 491;
    i32 nmp8: 4407;
    i32 nmp9: 265;
    i32 nmp10: 477;
    i32 nmp11: 1187;
    i32 nmp12: 917;
    i32 probcut1: 254;
    i32 probcut2: 85;
    i32 probcut3: 319;
    i32 probcut4: 197;
    i32 se1: 195;
    i32 se2: 48;
    i32 se3: 16;
    i32 se4: 16;
    i32 se5: 230;
    i32 se6: 56;
    i32 se7: 19;
    i32 se8: 15;
    i32 se9: 36;
    i32 ldse1: 7;
    i32 ldse2: 25;
    i32 lmp1: 2818;
    i32 lmp2: 78;
    i32 lmp3: 1351;
    i32 lmp4: 74;
    i32 fp1: 79;
    i32 fp2: 55;
    i32 fp3: 77;
    i32 fp4: 555;
    i32 fp5: 127;
    i32 fp6: 14;
    i32 bnfp1: 84;
    i32 bnfp2: 82;
    i32 bnfp3: 24;
    i32 bnfp4: 11;
    i32 hp1: 5;
    i32 hp2: 948;
    i32 see1: 12;
    i32 see2: 56;
    i32 see3: 27;
    i32 see4: 27;
    i32 see5: 7;
    i32 see6: 36;
    i32 see7: 39;
    i32 see8: 14;
    i32 lmr1: 269;
    i32 lmr2: 425;
    i32 lmr3: -241;
    i32 lmr4: 1155;
    i32 lmr5: 3417;
    i32 lmr6: 1412;
    i32 lmr7: 464;
    i32 lmr8: 326;
    i32 lmr9: 1024;
    i32 lmr10: 2171;
    i32 lmr11: 179;
    i32 lmr12: 418;
    i32 lmr13: -65;
    i32 lmr14: 91;
    i32 lmr15: 1426;
    i32 lmr16: 130;
    i32 lmr17: 519;
    i32 lmr18: 437;
    i32 lmr19: 333;
    i32 lmr20: 611;
    i32 lmr21: 685;
    i32 lmr22: 1852;
    i32 lmr23: 2204;
    i32 lmr24: 955;
    i32 lmr25: 1151;
    i32 lmr26: 400;
    i32 lmr27: 496;
    i32 lmr28: 185;
    i32 lmr29: 2021;
    i32 lmr30: 414;
    i32 lmr31: 136;
    u64 lmr32: 27;
    i32 lmr33: 59;
    i32 fds1: 207;
    i32 fds2: 366;
    i32 fds3: -206;
    i32 fds4: 1370;
    i32 fds5: 2255;
    i32 fds6: 1468;
    i32 fds7: 118;
    i32 fds8: 940;
    i32 fds9: 63;
    i32 fds10: 844;
    i32 fds11: 1129;
    i32 fds12: 1260;
    i32 fds13: 2168;
    i32 fds14: 1394;
    i32 fds15: 258;
    i32 fds16: 351;
    i32 fds17: 188;
    i32 fds18: 2167;
    i32 fds19: 3002;
    i32 fds20: 590;
    i32 fds21: 130;
    u64 fds22: 26;
    i32 fds23: 56;
    i32 fds24: 2621;
    i32 fds25: 5579;
    i32 dod1: 57;
    i32 dos1: 9;
    i32 noisy1: 96;
    i32 noisy2: 885;
    i32 noisy3: 43;
    i32 noisy4: 87;
    i32 noisy5: 175;
    i32 noisy6: 1252;
    i32 noisy7: 58;
    i32 noisy8: 16;
    i32 quiet1: 184;
    i32 quiet2: 1742;
    i32 quiet3: 72;
    i32 quiet4: 42;
    i32 quiet5: 171;
    i32 quiet6: 1099;
    i32 quiet7: 46;
    i32 quiet8: 31;
    i32 quiet9: 45;
    i32 cont1: 97;
    i32 cont2: 1098;
    i32 cont3: 74;
    i32 cont4: 48;
    i32 cont5: 414;
    i32 cont6: 949;
    i32 cont7: 49;
    i32 cont8: 17;
    i32 refut1: 93;
    i32 refut2: 52;
    i32 refut3: 935;
    i32 post1: 233;
    i32 post2: 86;
    i32 post3: 1550;
    i32 pcm1: 88;
    i32 pcm2: 17;
    i32 pcm3: 110;
    i32 pcm4: 144;
    i32 pcm5: 97;
    i32 pcm6: 306;
    i32 pcm7: 136;
    i32 pcm8: 180;
    i32 pcm9: 37;
    i32 pcm10: 2414;
    i32 pcm11: 152;
    i32 pcm12: 47;
    i32 pcm13: 1379;
    i32 pcm14: 50;
    i32 pcm15: 654;
    i32 pcm16: 229;
    i32 qs1: 8;
    i32 qs2: 68;
    i32 qs3: 74;
    i32 qs4: 100;
    i32 qs5: 48;
    i32 corrhist1: 64;
    i32 corrhist2: 148;
    i32 corrhist3: 4678;
    i32 corrhist4: 2496;
    i32 mp1: 1763;
    i32 mp2: 1614;
    i32 mp3: 1066;
    i32 mp4: 1086;
    i32 mp5: 1051;
    i32 mp6: 10723;
    i32 mp7: 8875;
    i32 mp8: 3446;
    i32 mp9: 4494;
    i32 mp10: 8854;
    i32 mp11: 8170;
    i32 mp12: 14051;
    i32 mp13: 20357;
    i32 mp14: 14232;
    i32 mp15: 4558;
    i32 mp16: 47;
    i32 mp17: 116;
    i32 mp18: 1024;
    i32 mp19: 2;
    i32 mp20: 1;
    i32 mp21: 200000;
    i32 mp22: 20000;
    i32 history1: 8192;
    i32 history2: 8192;
    i32 history3: 12800;
    i32 history4: 14605;
    i32 history5: 16418;
    i32 history6: 15320;
    f32 lerp1: 0.6945;
    f32 lerp2: 0.2695;
    f32 lerp3: 0.4027;
    f32 lerp5: 0.8256;
    f32 lerp6: 0.5072;
);
