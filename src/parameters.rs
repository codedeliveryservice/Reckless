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
    i32 delta2: 24342;
    i32 delta3: 29;
    i32 delta4: 56;
    i32 opt1: 132;
    i32 opt2: 249;
    i32 ttcut1: 146;
    i32 ttcut2: 62;
    i32 ttcut3: 1635;
    i32 ttcut4: 101;
    i32 ttcut5: 64;
    i32 ttcut6: 1588;
    i32 evalord1: 710;
    i32 evalord2: 120;
    i32 evalord3: 254;
    i32 hs1: 2617;
    i32 hs2: 945;
    i32 hs3: 63;
    i32 razor1: 301;
    i32 razor2: 259;
    i32 rfp1: 1309;
    i32 rfp2: 28;
    i32 rfp3: 69;
    i32 rfp4: 548;
    i32 rfp5: 31;
    i32 nmp1: 11;
    i32 nmp2: 151;
    i32 nmp3: 116;
    i32 nmp4: 236;
    i32 nmp5: 6308;
    i32 nmp6: 321;
    i32 probcut1: 252;
    i32 probcut2: 70;
    i32 se1: 289;
    i32 se2: 68;
    i32 se3: 329;
    i32 se4: 16;
    i32 red1: 482;
    i32 red2: 357;
    i32 red3: 1283;
    i32 lmp1: 16;
    i32 lmp2: 3578;
    i32 lmp3: 1037;
    i32 lmp4: 1571;
    i32 lmp5: 417;
    i32 fp1: 110;
    i32 fp2: 53;
    i32 fp3: 102;
    i32 fp4: 80;
    i32 bnfp1: 134;
    i32 bnfp2: 75;
    i32 bnfp3: 85;
    i32 bnfp4: 64;
    i32 see1: 2447;
    i32 see2: 34;
    i32 see3: 22;
    i32 see4: 87;
    i32 see5: 40;
    i32 see6: 42;
    i32 lmr1: 531;
    i32 lmr2: 137;
    i32 lmr3: 419;
    i32 lmr4: 106;
    i32 lmr5: 46;
    i32 lmr6: 3554;
    i32 lmr7: 69;
    i32 lmr8: 374;
    i32 lmr9: 624;
    i32 lmr10: 775;
    i32 lmr11: 380;
    i32 lmr12: 506;
    i32 lmr13: 1679;
    i32 lmr14: 1066;
    i32 lmr15: 1048;
    i32 lmr16: 1543;
    i32 lmr17: 759;
    i32 lmr18: 1247;
    i32 dod1: 36;
    i32 dod2: 452;
    i32 dos1: 15;
    i32 fds1: 365;
    i32 fds2: 157;
    i32 fds3: 335;
    i32 fds4: 65;
    i32 fds5: 44;
    i32 fds6: 2546;
    i32 fds7: 54;
    i32 fds8: 731;
    i32 fds9: 1115;
    i32 fds10: 1316;
    i32 fds11: 1008;
    i32 fds12: 1335;
    i32 fds13: 1096;
    i32 fds14: 3006;
    i32 noisy1: 127;
    i32 noisy2: 54;
    i32 noisy3: 1103;
    i32 noisy4: 71;
    i32 noisy5: 157;
    i32 noisy6: 53;
    i32 noisy7: 1458;
    i32 noisy8: 23;
    i32 quiet1: 157;
    i32 quiet2: 67;
    i32 quiet3: 1614;
    i32 quiet4: 64;
    i32 quiet5: 122;
    i32 quiet6: 49;
    i32 quiet7: 1150;
    i32 quiet8: 40;
    i32 cont1: 97;
    i32 cont2: 59;
    i32 cont3: 1180;
    i32 cont4: 69;
    i32 cont5: 332;
    i32 cont6: 48;
    i32 cont7: 908;
    i32 cont8: 24;
    i32 refut1: 81;
    i32 refut2: 50;
    i32 refut3: 764;
    i32 post1: 232;
    i32 post2: 92;
    i32 post3: 1437;
    i32 pcm1: 80;
    i32 pcm2: 144;
    i32 pcm3: 204;
    i32 pcm4: 120;
    i32 pcm5: 193;
    i32 pcm6: 130;
    i32 pcm7: 289;
    i32 pcm8: 104;
    i32 pcm9: 159;
    i32 pcm10: 42;
    i32 pcm11: 2090;
    i32 pcm12: 164;
    i32 pcm13: 41;
    i32 pcm14: 1473;
    i32 qs1: 81;
    i32 qs2: 31;
    i32 corrhist1: 157;
    i32 corrhist2: 4359;
    i32 corrhist3: 2858;
);
