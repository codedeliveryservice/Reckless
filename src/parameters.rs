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
    i32 delta2: 25021;
    i32 delta3: 27;
    i32 delta4: 57;
    i32 opt1: 145;
    i32 opt2: 229;
    i32 ttcut1: 174;
    i32 ttcut2: 67;
    i32 ttcut3: 1508;
    i32 ttcut4: 109;
    i32 ttcut5: 58;
    i32 ttcut6: 1456;
    i32 ttpcm1: 90;
    i32 ttpcm2: 158;
    i32 ttpcm3: 125;
    i32 ttpcm4: 327;
    i32 ttpcm5: 97;
    i32 ttpcm6: 164;
    i32 ttpcm7: 40;
    i32 ttpcm8: 2143;
    i32 ttpcm9: 152;
    i32 ttpcm10: 39;
    i32 ttpcm11: 1486;
    i32 evalord1: 748;
    i32 evalord2: 123;
    i32 evalord3: 257;
    i32 hs1: 2505;
    i32 hs2: 940;
    i32 hs3: 61;
    i32 razor1: 276;
    i32 razor2: 266;
    i32 rfp1: 1220;
    i32 rfp2: 27;
    i32 rfp3: 70;
    i32 rfp4: 499;
    i32 rfp5: 31;
    i32 nmp1: 9;
    i32 nmp2: 134;
    i32 nmp3: 128;
    i32 nmp4: 269;
    i32 nmp5: 6308;
    i32 nmp6: 321;
    i32 probcut1: 254;
    i32 probcut2: 73;
    i32 se1: 259;
    i32 se2: 54;
    i32 se3: 341;
    i32 se4: 16;
    i32 redbase1: 1300;
    i32 redbase2: 286;
    i32 red1: 446;
    i32 red2: 304;
    i32 red3: 1288;
    i32 red4: 439;
    i32 red5: 404;
    i32 lmp1: 16;
    i32 lmp2: 3318;
    i32 lmp3: 1078;
    i32 lmp4: 1323;
    i32 lmp5: 350;
    i32 fp1: 99;
    i32 fp2: 59;
    i32 fp3: 102;
    i32 fp4: 75;
    i32 bnfp1: 116;
    i32 bnfp2: 61;
    i32 bnfp3: 87;
    i32 bnfp4: 74;
    i32 see1: 2003;
    i32 see2: 32;
    i32 see3: 24;
    i32 see4: 89;
    i32 see5: 36;
    i32 see6: 42;
    i32 lmr1: 543;
    i32 lmr2: 156;
    i32 lmr3: 343;
    i32 lmr4: 97;
    i32 lmr5: 52;
    i32 lmr6: 3439;
    i32 lmr7: 67;
    i32 lmr8: 365;
    i32 lmr9: 672;
    i32 lmr10: 803;
    i32 lmr13: 1883;
    i32 lmr14: 1139;
    i32 lmr15: 938;
    i32 lmr16: 1570;
    i32 lmr17: 626;
    i32 lmr18: 1291;
    i32 lmr19: 937;
    i32 dod1: 37;
    i32 dod2: 429;
    i32 dos1: 15;
    i32 fds1: 348;
    i32 fds2: 141;
    i32 fds3: 307;
    i32 fds4: 66;
    i32 fds5: 47;
    i32 fds6: 2607;
    i32 fds7: 51;
    i32 fds8: 749;
    i32 fds9: 1038;
    i32 fds10: 1463;
    i32 fds11: 1208;
    i32 fds12: 1470;
    i32 fds13: 1092;
    i32 fds14: 3028;
    i32 fdsred: 5454;
    i32 noisy1: 108;
    i32 noisy2: 51;
    i32 noisy3: 998;
    i32 noisy4: 79;
    i32 noisy5: 166;
    i32 noisy6: 61;
    i32 noisy7: 1421;
    i32 noisy8: 24;
    i32 quiet1: 157;
    i32 quiet2: 67;
    i32 quiet3: 1389;
    i32 quiet4: 67;
    i32 quiet5: 127;
    i32 quiet6: 47;
    i32 quiet7: 1067;
    i32 quiet8: 41;
    i32 cont1: 99;
    i32 cont2: 66;
    i32 cont3: 1020;
    i32 cont4: 67;
    i32 cont5: 355;
    i32 cont6: 47;
    i32 cont7: 1007;
    i32 cont8: 21;
    i32 refut1: 90;
    i32 refut2: 57;
    i32 refut3: 814;
    i32 post1: 191;
    i32 post2: 84;
    i32 post3: 1522;
    i32 pcm1: 90;
    i32 pcm2: 158;
    i32 pcm3: 198;
    i32 pcm4: 125;
    i32 pcm5: 195;
    i32 pcm6: 101;
    i32 pcm7: 327;
    i32 pcm8: 97;
    i32 pcm9: 164;
    i32 pcm10: 40;
    i32 pcm11: 2143;
    i32 pcm12: 152;
    i32 pcm13: 39;
    i32 pcm14: 1486;
    i32 qs1: 92;
    i32 qs2: 35;
    i32 corrhist1: 182;
    i32 corrhist2: 4838;
    i32 corrhist3: 2985;
    i32 corrhist4: 76;
    i32 mp1: 36;
    i32 mp2: 119;
    i32 eval1: 21366;
    i32 eval2: 1747;
    i32 eval3: 27395;
    i32 shawnofthewalk: 42;
);
