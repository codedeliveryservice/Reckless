use crate::lmr::*;

pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

pub fn set_parameter(name: &str, value: &str) {
    if let Some(idx_str) = name.strip_prefix("single_") {
        let idx = idx_str.parse::<usize>().unwrap();
        let val = value.parse::<i32>().unwrap();
        unsafe { SINGLE_VALUES[idx] = val };
    } else if let Some(idx_str) = name.strip_prefix("double_") {
        let idx = idx_str.parse::<usize>().unwrap();
        let val = value.parse::<i32>().unwrap();
        unsafe { DOUBLE_VALUES[idx] = val };
    }
}

pub fn print_options() {
    for i in 0..SINGLE_VALUES_LEN {
        println!("option name single_{i} type string");
    }

    for i in 0..DOUBLE_VALUES_LEN {
        println!("option name double_{i} type string");
    }
}

pub fn print_params() {
    for i in 0..SINGLE_VALUES_LEN {
        println!("single_{i}, int, {}, -2048, 2048, 384, 0.002", unsafe { SINGLE_VALUES[i] });
    }

    for i in 0..DOUBLE_VALUES_LEN {
        println!("double_{i}, int, {}, -2048, 2048, 224, 0.002", unsafe { DOUBLE_VALUES[i] });
    }
}
