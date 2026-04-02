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
    i32 eval1: 21061;
    i32 eval2: 1519;
    i32 eval3: 26556;
    i32 mp1: 46;
    i32 mp2: 109;
    i32 mp3: 16;
    i32 mp4: 8000;
    i32 mp5: 8000;
    i32 mp6: 14000;
    i32 mp7: 20000;
    i32 mp8: 10000;
    i32 mp9: 8000;
    i32 mp10: 6000;
    i32 delta1: 13;
    i32 delta2: 23660;
    i32 delta3: 27;
    i32 delta4: 63;
    i32 opt1: 169;
    i32 opt2: 187;
    i32 ttcut1: 185;
    i32 ttcut2: 81;
    i32 ttcut3: 1806;
    i32 ttcut4: 108;
    i32 ttcut5: 56;
    i32 ttcut6: 1365;
    i32 ttcut7: 4;
    i32 evalord1: 819;
    i32 evalord2: 124;
    i32 evalord3: 312;
    i32 hs1: 2247;
    i32 hs2: 59;
    i32 razor1: 299;
    i32 razor2: 252;
    i32 rfp1: 1125;
    i32 rfp2: 26;
    i32 rfp3: 77;
    i32 rfp4: 519;
    i32 rfp5: 64;
    i32 rfp6: 32;
    i32 nmp1: 9;
    i32 nmp2: 126;
    i32 nmp3: 128;
    i32 nmp4: 286;
    i32 nmp5: 20;
    i32 nmp6: 5154;
    i32 nmp7: 271;
    i32 nmp8: 535;
    i32 nmp9: 1073;
    i32 probcut1: 269;
    i32 probcut2: 72;
    i32 probcut3: 295;
    i32 probcut4: 282;
    i32 se1: 200;
    i32 se2: 16;
    i32 se3: 16;
    i32 se4: 288;
    i32 se5: 16;
    i32 se6: 16;
    i32 se7: 32;
    i32 lmp1: 3072;
    i32 lmp2: 64;
    i32 lmp3: 1536;
    i32 lmp4: 64;
    i32 fp1: 88;
    i32 fp2: 63;
    i32 fp3: 88;
    i32 fp4: 114;
    i32 fp5: 14;
    i32 bnfp1: 71;
    i32 bnfp2: 69;
    i32 bnfp3: 25;
    i32 bnfp4: 12;
    i32 see1: 16;
    i32 see2: 52;
    i32 see3: 21;
    i32 see4: 22;
    i32 see5: 8;
    i32 see6: 36;
    i32 see7: 32;
    i32 see8: 11;
    i32 lmr1: 250;
    i32 lmr2: 65;
    i32 lmr3: 3183;
    i32 lmr4: 1300;
    i32 lmr5: 600;
    i32 lmr6: 300;
    i32 lmr7: 1875;
    i32 lmr8: 154;
    i32 lmr9: 1355;
    i32 lmr10: 109;
    i32 lmr11: 411;
    i32 lmr12: 421;
    i32 lmr13: 371;
    i32 lmr14: 656;
    i32 lmr15: 824;
    i32 lmr16: 1762;
    i32 lmr17: 2116;
    i32 lmr18: 438;
    i32 lmr19: 279;
    i32 lmr20: 1288;
    i32 lmr21: 966;
    i32 lmr22: 1604;
    i32 lmr23: 128;
    i32 dod1: 60;
    i32 dod2: 768;
    i32 dos1: 5;
    i32 fds1: 238;
    i32 fds2: 57;
    i32 fds3: 2513;
    i32 fds4: 1427;
    i32 fds5: 158;
    i32 fds6: 1098;
    i32 fds7: 65;
    i32 fds8: 897;
    i32 fds9: 1127;
    i32 fds10: 1450;
    i32 fds11: 2200;
    i32 fds12: 454;
    i32 fds13: 254;
    i32 fds14: 1368;
    i32 fds15: 1452;
    i32 fds16: 3316;
    i32 fds17: 128;
    i32 fdsred1: 3072;
    i32 fdsred2: 5687;
    i32 noisy1: 106;
    i32 noisy2: 54;
    i32 noisy3: 808;
    i32 noisy4: 80;
    i32 noisy5: 164;
    i32 noisy6: 52;
    i32 noisy7: 1329;
    i32 noisy8: 23;
    i32 quiet1: 172;
    i32 quiet2: 78;
    i32 quiet3: 1459;
    i32 quiet4: 54;
    i32 quiet5: 144;
    i32 quiet6: 45;
    i32 quiet7: 1064;
    i32 quiet8: 39;
    i32 cont1: 108;
    i32 cont2: 67;
    i32 cont3: 977;
    i32 cont4: 52;
    i32 cont5: 352;
    i32 cont6: 47;
    i32 cont7: 868;
    i32 cont8: 19;
    i32 refut1: 86;
    i32 refut2: 60;
    i32 refut3: 771;
    i32 post1: 201;
    i32 post2: 86;
    i32 post3: 1634;
    i32 pcm1: 95;
    i32 pcm2: 156;
    i32 pcm3: 215;
    i32 pcm4: 113;
    i32 pcm5: 130;
    i32 pcm6: 96;
    i32 pcm7: 317;
    i32 pcm8: 120;
    i32 pcm9: 153;
    i32 pcm10: 34;
    i32 pcm11: 2474;
    i32 pcm12: 156;
    i32 pcm13: 38;
    i32 pcm14: 1169;
    i32 pcm15: 60;
    i32 pcm16: 5;
    i32 pcm17: 8;
    i32 qs1: 8;
    i32 qs2: 100;
    i32 corrhist1: 77;
    i32 corrhist2: 142;
    i32 corrhist3: 4923;
    i32 corrhist4: 3072;
);
