pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

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
    i32 delta2: 27153;
    i32 delta3: 43;
    i32 delta4: 15;
    i32 optimism1: 111;
    i32 optimism2: 234;
    i32 tt_cut1: 5;
    i32 tt_cut2: 135;
    i32 tt_cut3: 68;
    i32 tt_cut4: 1409;
    i32 tt_cut5: 102;
    i32 tt_cut6: 62;
    i32 tt_cut7: 1457;
    i32 static1: 698;
    i32 static2: 61;
    i32 static3: 139;
    i32 hs1: 2689;
    i32 hs2: 937;
    i32 hs3: 62;
    i32 razor1: 299;
    i32 razor2: 258;
    i32 rfp1: 7;
    i32 rfp2: 74;
    i32 rfp3: 70;
    i32 rfp4: 23;
    i32 rfp5: 579;
    i32 rfp6: 23;
    i32 nmp1: 15;
    i32 nmp2: 149;
    i32 nmp3: 108;
    i32 nmp4: 181;
    i32 nmp5: 250;
    i32 probcut1: 277;
    i32 probcut2: 61;
    i32 r1: 493;
    i32 r2: 427;
    i32 r3: 1208;
    i32 r4: 140;
    i32 lmp1: 16;
    i32 fp1: 123;
    i32 fp2: 78;
    i32 fp3: 35;
    i32 fp4: 9;
    i32 bnfp1: 115;
    i32 bnfp2: 389;
    i32 bnfp3: 81;
    i32 bnfp4: 478;
    i32 bnfp5: 84;
    i32 bnfp6: 6;
    i32 see1: 22;
    i32 see2: 45;
    i32 see3: 0;
    i32 see4: 93;
    i32 see5: 48;
    i32 see6: 43;
    i32 see7: 2;
    i32 se1: 63;
    i32 se2: 14;
    i32 lmr1: 102;
    i32 lmr2: 599;
    i32 lmr3: 99;
    i32 lmr4: 564;
    i32 lmr5: 3225;
    i32 lmr6: 55;
    i32 lmr7: 312;
    i32 lmr8: 700;
    i32 lmr9: 637;
    i32 lmr10: 822;
    i32 lmr11: 800;
    i32 lmr12: 598;
    i32 lmr13: 553;
    i32 lmr14: 35;
    i32 lmr15: 1149;
    i32 lmr16: 801;
    i32 lmr17: 1180;
    i32 dod1: 48;
    i32 dod2: 527;
    i32 post1: 157;
    i32 post2: 52;
    i32 post3: 1030;
    i32 raise1: 16;
    i32 hist1: 126;
    i32 hist2: 63;
    i32 hist3: 1156;
    i32 hist4: 141;
    i32 hist5: 71;
    i32 hist6: 1444;
    i32 hist7: 14;
    i32 hist8: 156;
    i32 hist9: 70;
    i32 hist10: 1591;
    i32 hist11: 66;
    i32 hist12: 131;
    i32 hist13: 54;
    i32 hist14: 1286;
    i32 hist15: 17;
    i32 hist16: 203;
    i32 hist17: 101;
    i32 hist18: 57;
    i32 hist19: 1311;
    i32 hist20: 70;
    i32 hist21: 268;
    i32 hist22: 50;
    i32 hist23: 974;
    i32 hist24: 14;
    i32 hist25: 129;
    i32 pcm1: 107;
    i32 pcm2: 132;
    i32 pcm3: 224;
    i32 pcm4: 131;
    i32 pcm5: 276;
    i32 pcm6: 101;
    i32 pcm7: 140;
    i32 pcm8: 43;
    i32 pcm9: 1677;
    i32 qs1: 130;
    i32 qs2: 73;
    i32 corr1: 139;
    i32 corr2: 4090;
    i32 corr3: 3265;
    i32 corr4: 91;
    i32 corr5: 100;
    i32 eval1: 20545;
    i32 eval2: 1773;
    i32 eval3: 29892;
    i32 mp1: 37;
    i32 mp2: 104;
    i32 mp3: 2163;
    i32 mp4: 1007;
    i32 max1: 2002;
    i32 max2: 6236;
    i32 max3: 4403;
    i32 max4: 8205;
    i32 max5: 15369;
    i32 max6: 16070;
    i32 max7: 15543;
);
