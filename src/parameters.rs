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
    i32 nb_w0: 106;
    i32 nb_w1: 0;
    i32 nb_w2: 0;
    i32 nb_w3: 0;
    i32 nb_w4: 0;
    i32 nb_w5: 0;
    i32 nb_w6: 0;
    i32 nb_w7: -80;
    i32 nb_w8: -54;
    i32 nb_clamp: 808;

    i32 nm_w0: 164;
    i32 nm_w1: 0;
    i32 nm_w2: -23;
    i32 nm_w3: 0;
    i32 nm_w4: 0;
    i32 nm_w5: 0;
    i32 nm_w6: 0;
    i32 nm_w7: 0;
    i32 nm_w8: -52;
    i32 nm_clamp: 1329;

    i32 qb_w0: 172;
    i32 qb_w1: 0;
    i32 qb_w2: 0;
    i32 qb_w3: 0;
    i32 qb_w4: 0;
    i32 qb_w5: 0;
    i32 qb_w6: 0;
    i32 qb_w7: -54;
    i32 qb_w8: -78;
    i32 qb_clamp: 1459;

    i32 qm_w0: 144;
    i32 qm_w1: -39;
    i32 qm_w2: 0;
    i32 qm_w3: 0;
    i32 qm_w4: 0;
    i32 qm_w5: 0;
    i32 qm_w6: 0;
    i32 qm_w7: 0;
    i32 qm_w8: -45;
    i32 qm_clamp: 1064;

    i32 cb_w0: 108;
    i32 cb_w1: 0;
    i32 cb_w2: 0;
    i32 cb_w3: 0;
    i32 cb_w4: 0;
    i32 cb_w5: 0;
    i32 cb_w6: 0;
    i32 cb_w7: -52;
    i32 cb_w8: -67;
    i32 cb_clamp: 977;

    i32 cm_w0: 352;
    i32 cm_w1: -19;
    i32 cm_w2: 0;
    i32 cm_w3: 0;
    i32 cm_w4: 0;
    i32 cm_w5: 0;
    i32 cm_w6: 0;
    i32 cm_w7: 0;
    i32 cm_w8: -47;
    i32 cm_clamp: 868;
);
