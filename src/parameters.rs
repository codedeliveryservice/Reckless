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
    i32 fds_r1: 3398;
    i32 fds_r2: 54;
    i32 fds_r3: 343;
    i32 fds_r4: 662;
    i32 fds_r5: 665;
    i32 fds_r6: 796;
    i32 fds_r7: 838;
    i32 fds_r8: 601;
    i32 fds_r9: 582;
    i32 fds_r10: 33;
    i32 fds_r11: 1247;
    i32 fds_r12: 927;
    i32 fds_r13: 800;
    i32 fds_r14: 1270;
    i32 fds_r15: 752;
    i32 fds_r16: 1057;
    i32 fds_r17: 2048;
    i32 fds_r18: 107;
    i32 fds_r19: 572;
    i32 fds_r20: 99;
    i32 fds_r21: 561;
    i32 fds_r22: 40;
    i32 fds_r23: 120;
    i32 fds_r24: 1024;
);
