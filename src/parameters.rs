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
    -0.235930, -0.007297, 0.212446, 0.364875, 0.071978, -0.123717, 0.005488, 0.450056, -0.295969, 0.168642, 0.157221,
    -0.093035, 0.107610, -0.096240, 0.144838, -0.103337, 0.160000, 0.013998, 0.007580, -0.003271, -0.067188, -0.021317,
    -0.047155, -0.204262, -0.111886, -0.220198, 0.094137, 0.143638, 0.057093, 0.090023, 0.380414, 0.309352, 0.186897,
    -0.192671, 0.198730, 0.181176, 0.194120, 0.209486, 0.311636, 0.009723, 0.353998, 0.341565, -0.003718, 0.360775,
    0.356818, 0.549502, 0.047653, 0.440257, 0.290695, 0.381591, 0.345512, 0.384273, 0.037824, 0.180434, 0.124946,
    -0.212259, 0.122185, 0.062661, 0.102262, 0.002221, 0.068853, 0.101385, 0.108392, 0.137625, 0.214630, 0.117022,
    -0.112002, 0.091152, -0.005555, -0.171481, 0.040869, -0.068335, -0.021452, -0.198139, -0.044157, -0.042768,
    -0.188053, -0.118784, 0.005712, -0.115260, 0.105081, -0.081699, -0.092076, -0.005787, -0.029814, 0.013293,
    -0.020203, -0.028776, 0.021409, -0.036272, -0.059014, -0.000556, 0.000857, 0.001004, 0.004706, -0.777761,
    -0.029945, -0.031320, 0.829728, -0.102942, 0.378867, 0.001837, 0.001890, 0.001192,
];
const FC1_BIAS: [f32; 8] = [0.548648, 1.088406, -0.234875, -0.655781, -0.207737, -0.235942, -0.145490, -0.043466];
const FC2_WEIGHT: [f32; 8] = [0.369022, 0.920194, -0.174451, -0.666244, -0.161927, 0.103019, -0.009633, 0.592598];
const FC2_BIAS: f32 = 0.416372;

const Y_MEAN: f32 = -2638.217285;
const Y_STD: f32 = 1878.540161;

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
