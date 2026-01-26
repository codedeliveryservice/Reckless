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
    i32 nb_w0: 134;
    i32 nb_w1: -31;
    i32 nb_w2: 85;
    i32 nb_w3: 6;
    i32 nb_w4: 126;
    i32 nb_w5: -21;
    i32 nb_w6: -11;
    i32 nb_w7: -79;
    i32 nb_w8: 84;
    i32 nb_clamp: 779;

    i32 nm_w0: 179;
    i32 nm_w1: -25;
    i32 nm_w2: -31;
    i32 nm_w3: 57;
    i32 nm_w4: -5;
    i32 nm_w5: -40;
    i32 nm_w6: -103;
    i32 nm_w7: -99;
    i32 nm_w8: -134;
    i32 nm_clamp: 1425;

    i32 qb_w0: 215;
    i32 qb_w1: -52;
    i32 qb_w2: 118;
    i32 qb_w3: 39;
    i32 qb_w4: 68;
    i32 qb_w5: -20;
    i32 qb_w6: 59;
    i32 qb_w7: -37;
    i32 qb_w8: -24;
    i32 qb_clamp: 1430;

    i32 qm_w0: 180;
    i32 qm_w1: -119;
    i32 qm_w2: 124;
    i32 qm_w3: 89;
    i32 qm_w4: 6;
    i32 qm_w5: -80;
    i32 qm_w6: -21;
    i32 qm_w7: -12;
    i32 qm_w8: -5;
    i32 qm_clamp: 1043;

    i32 cb_w0: 109;
    i32 cb_w1: -28;
    i32 cb_w2: 40;
    i32 cb_w3: 20;
    i32 cb_w4: -29;
    i32 cb_w5: -50;
    i32 cb_w6: 29;
    i32 cb_w7: -137;
    i32 cb_w8: -84;
    i32 cb_clamp: 1043;

    i32 cm_w0: 401;
    i32 cm_w1: -33;
    i32 cm_w2: 30;
    i32 cm_w3: -27;
    i32 cm_w4: 23;
    i32 cm_w5: 23;
    i32 cm_w6: -85;
    i32 cm_w7: 106;
    i32 cm_w8: -129;
    i32 cm_clamp: 839;
);
