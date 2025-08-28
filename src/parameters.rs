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
    i32 lmr_v1: 107;
    i32 lmr_v2: 572;
    i32 lmr_v3: 99;
    i32 lmr_v4: 561;
    i32 lmr_v5: 40;
    i32 lmr_v6: 120;
    i32 lmr_v7: 3398;
    i32 lmr_v8: 54;
    i32 lmr_v9: 343;
    i32 lmr_v10: 662;
    i32 lmr_v11: 665;
    i32 lmr_v12: 796;
    i32 lmr_v13: 838;
    i32 lmr_v14: 601;
    i32 lmr_v15: 582;
    i32 lmr_v16: 1247;
    i32 lmr_v17: 927;
    i32 lmr_v18: 800;
    i32 lmr_v19: 1270;
    i32 lmr_v20: 752;
    i32 lmr_v21: 1057;
    i32 lmr_v22: 45;
    i32 lmr_v23: 565;
    i32 lmr_v24: 128;
    i32 lmr_v25: 152;
    i32 lmr_v26: 48;
    i32 lmr_v27: 1027;
    i32 fds_r1: 3128;
    i32 fds_r2: 51;
    i32 fds_r3: 361;
    i32 fds_r4: 626;
    i32 fds_r5: 683;
    i32 fds_r6: 897;
    i32 fds_r7: 875;
    i32 fds_r8: 631;
    i32 fds_r9: 603;
    i32 fds_r10: 34;
    i32 fds_r11: 1253;
    i32 fds_r12: 986;
    i32 fds_r13: 780;
    i32 fds_r14: 1304;
    i32 fds_r15: 788;
    i32 fds_r16: 1033;
    i32 fds_r17: 2071;
    i32 fds_r18: 112;
    i32 fds_r19: 592;
    i32 fds_r20: 100;
    i32 fds_r21: 615;
    i32 fds_r22: 40;
    i32 fds_r23: 116;
    i32 fds_r24: 864;
);
