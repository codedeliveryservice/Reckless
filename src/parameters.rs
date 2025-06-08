use std::arch::x86_64::*;
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

const FC1_WEIGHT: [f32; 104] = [
    0.068313, 0.015755, 0.110732, 0.350910, -0.257920, -0.097214, -0.135447, 0.182349, 0.097637, 0.006475, -0.083817,
    -0.164598, -0.095950, 0.004949, 0.228003, 0.163535, 0.158164, 0.369438, 0.135269, 0.081171, -0.040963, 0.163508,
    0.103360, 0.215858, 0.223749, 0.266888, -0.001677, -0.007218, -0.011183, 0.073182, -0.488597, -0.289416, -0.377567,
    0.196410, -0.193716, -0.304960, -0.039358, -0.065709, -0.013539, -0.092878, 0.182085, 0.215492, 0.210194, 0.108960,
    -0.156771, -0.188566, 0.138590, -0.229610, -0.000951, -0.083511, 0.108245, -0.163557, -0.237814, 0.100580,
    0.039673, 0.270677, 0.040831, -0.019980, -0.062718, 0.336134, -0.198091, 0.206172, 0.025001, 0.023231, 0.064095,
    0.012408, 0.173253, 0.443922, 0.283764, 0.525316, 0.109998, 0.149350, -0.153987, 0.109825, 0.075323, 0.291905,
    0.228770, 0.383821, 0.152953, 0.226680, 0.315070, 0.009389, 0.095926, 0.198373, 0.223194, -0.041746, 0.099981,
    0.183008, 0.196463, 0.223938, 0.163598, -0.000875, -0.002633, -0.003310, 0.000832, 0.617495, 0.004622, 0.005888,
    -0.622707, -0.020600, -0.517819, -0.011329, -0.013467, -0.006868,
];

const FC1_BIAS: [f32; 8] = [0.820603, -0.472442, 0.212730, 0.392578, 0.557809, -0.608154, -0.308795, 0.032281];
const FC2_WEIGHT: [f32; 8] = [0.841000, -0.499201, 0.187101, 0.301042, 0.713962, -0.598537, -0.353043, 0.567683];
const FC2_BIAS: f32 = 0.149023;

const Y_MEAN: f32 = -2637.521484;
const Y_STD: f32 = 1877.284058;

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 13;

#[target_feature(enable = "avx2")]
pub unsafe fn lmr_forward(mut input: [f32; INPUT_SIZE]) -> f32 {
    input[0] = (input[0] + 593.73) / 4011.14;
    input[1] /= 92.83;
    input[2] /= 9.53;
    input[3] /= 3.78;

    let mut hidden = [0.0f32; HIDDEN_SIZE];

    for i in 0..HIDDEN_SIZE {
        let weight_offset = i * INPUT_SIZE;
        let w_ptr = FC1_WEIGHT[weight_offset..].as_ptr();
        let in_ptr = input.as_ptr();

        let mut sum = _mm256_setzero_ps();

        let len = INPUT_SIZE;
        let mut j = 0;

        while j + 8 <= len {
            let w = _mm256_loadu_ps(w_ptr.add(j));
            let x = _mm256_loadu_ps(in_ptr.add(j));
            sum = _mm256_fmadd_ps(w, x, sum);
            j += 8;
        }

        let mut acc = _mm256_hadd_ps(sum, sum);
        acc = _mm256_hadd_ps(acc, acc);
        let sum_arr: [f32; 8] = std::mem::transmute(acc);
        let mut output = sum_arr[0] + sum_arr[4];

        while j < len {
            output += *w_ptr.add(j) * *in_ptr.add(j);
            j += 1;
        }

        output += FC1_BIAS[i];
        hidden[i] = if output > 0.0 { output } else { 0.0 };
    }

    let w2_ptr = FC2_WEIGHT.as_ptr();
    let h_ptr = hidden.as_ptr();

    let w2 = _mm256_loadu_ps(w2_ptr);
    let h = _mm256_loadu_ps(h_ptr);
    let sum = _mm256_mul_ps(w2, h);

    let mut acc = _mm256_hadd_ps(sum, sum);
    acc = _mm256_hadd_ps(acc, acc);
    let sum_arr: [f32; 8] = std::mem::transmute(acc);
    let mut y = sum_arr[0] + sum_arr[4];

    y += FC2_BIAS;
    y * Y_STD + Y_MEAN
}
