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
    i32 delta2: 24373;
    i32 delta3: 26;
    i32 delta4: 53;
    i32 opt1: 131;
    i32 opt2: 246;
    i32 ttcut1: 158;
    i32 ttcut2: 57;
    i32 ttcut3: 1596;
    i32 ttcut4: 102;
    i32 ttcut5: 62;
    i32 ttcut6: 1618;
    i32 evalord1: 733;
    i32 evalord2: 114;
    i32 evalord3: 251;
    i32 hs1: 2606;
    i32 hs2: 945;
    i32 hs3: 59;
    i32 razor1: 278;
    i32 razor2: 257;
    i32 rfp1: 1192;
    i32 rfp2: 29;
    i32 rfp3: 73;
    i32 rfp4: 562;
    i32 rfp5: 31;
    i32 nmp1: 11;
    i32 nmp2: 140;
    i32 nmp3: 113;
    i32 nmp4: 252;
    i32 probcut1: 237;
    i32 probcut2: 72;
    i32 se1: 276;
    i32 se2: 66;
    i32 se3: 325;
    i32 se4: 16;
    i32 redbase1: 1200;
    i32 redbase2: 264;
    i32 red1: 461;
    i32 red2: 321;
    i32 red3: 1373;
    i32 lmp1: 16;
    i32 lmp2: 3525;
    i32 lmp3: 1056;
    i32 lmp4: 1426;
    i32 lmp5: 424;
    i32 fp1: 102;
    i32 fp2: 56;
    i32 fp3: 103;
    i32 fp4: 81;
    i32 bnfp1: 132;
    i32 bnfp2: 65;
    i32 bnfp3: 87;
    i32 bnfp4: 68;
    i32 see1: 2435;
    i32 see2: 30;
    i32 see3: 22;
    i32 see4: 89;
    i32 see5: 40;
    i32 see6: 45;
    i32 lmr1: 542;
    i32 lmr2: 142;
    i32 lmr3: 368;
    i32 lmr4: 107;
    i32 lmr5: 48;
    i32 lmr6: 3507;
    i32 lmr7: 73;
    i32 lmr8: 363;
    i32 lmr9: 686;
    i32 lmr10: 784;
    i32 lmr11: 395;
    i32 lmr12: 515;
    i32 lmr13: 1772;
    i32 lmr14: 1078;
    i32 lmr15: 1007;
    i32 lmr16: 1546;
    i32 lmr17: 752;
    i32 lmr18: 1214;
    i32 lmr19: 1024;
    i32 dod1: 36;
    i32 dod2: 447;
    i32 dos1: 15;
    i32 fds1: 359;
    i32 fds2: 156;
    i32 fds3: 337;
    i32 fds4: 68;
    i32 fds5: 45;
    i32 fds6: 2572;
    i32 fds7: 50;
    i32 fds8: 718;
    i32 fds9: 1074;
    i32 fds10: 1375;
    i32 fds11: 1128;
    i32 fds12: 1392;
    i32 fds13: 1138;
    i32 fds14: 2984;
    i32 noisy1: 116;
    i32 noisy2: 52;
    i32 noisy3: 1134;
    i32 noisy4: 69;
    i32 noisy5: 151;
    i32 noisy6: 54;
    i32 noisy7: 1380;
    i32 noisy8: 24;
    i32 quiet1: 160;
    i32 quiet2: 66;
    i32 quiet3: 1527;
    i32 quiet4: 60;
    i32 quiet5: 131;
    i32 quiet6: 48;
    i32 quiet7: 1116;
    i32 quiet8: 40;
    i32 cont1: 97;
    i32 cont2: 62;
    i32 cont3: 1129;
    i32 cont4: 69;
    i32 cont5: 341;
    i32 cont6: 47;
    i32 cont7: 948;
    i32 cont8: 23;
    i32 refut1: 77;
    i32 refut2: 50;
    i32 refut3: 817;
    i32 post1: 238;
    i32 post2: 86;
    i32 post3: 1500;
    i32 pcm1: 82;
    i32 pcm2: 150;
    i32 pcm3: 205;
    i32 pcm4: 124;
    i32 pcm5: 190;
    i32 pcm6: 128;
    i32 pcm7: 298;
    i32 pcm8: 100;
    i32 pcm9: 160;
    i32 pcm10: 43;
    i32 pcm11: 2124;
    i32 pcm12: 166;
    i32 pcm13: 42;
    i32 pcm14: 1495;
    i32 qs1: 84;
    i32 qs2: 33;
    i32 corrhist1: 164;
    i32 corrhist2: 4607;
    i32 corrhist3: 2647;
    i32 corrhist4: 90;
);
