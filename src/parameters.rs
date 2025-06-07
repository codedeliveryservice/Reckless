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

fn relu(x: f32) -> f32 {
    if x > 0.0 {
        x
    } else {
        0.0
    }
}

const FC1_WEIGHT: [f32; 104] = [
    0.007362, 0.168413, 0.274671, 0.137478, 0.380552, -0.003706, 0.051949, -0.122631, 0.180883, 0.102694, 0.197603, 0.116325, 0.165675,
    -0.000543, -0.000288, -0.000378, -0.001142, 1.259012, -0.369488, -0.453419, -0.440387, -0.016561, 0.014386, -0.002813, -0.002147, -0.002135,
    0.029055, 0.301576, 0.176181, 0.005513, 0.444576, 0.110555, 0.205859, 0.020442, 0.170089, 0.155703, 0.352527, 0.301357, 0.397965,
    0.070711, 0.249596, 0.259692, 0.006622, 0.767045, 0.218360, 0.218852, -0.172881, 0.380728, 0.246253, 0.147029, 0.283069, 0.350570,
    -0.223930, 0.087688, 0.208027, 0.143679, -0.734394, 0.314693, 0.283507, 0.496607, 0.021395, 0.134617, -0.048748, 0.093851, 0.022525,
    0.016505, 0.117419, 0.046631, -0.146664, 0.159141, -0.016512, -0.008243, -0.143072, 0.002406, -0.010326, -0.011541, -0.003682, -0.009147,
    -0.064574, 0.351799, -0.021491, 0.500016, -0.525388, 0.141220, 0.285497, 0.431342, -0.119255, 0.130823, -0.115592, -0.114268, -0.000275,
    0.006517, -0.028573, -0.027091, 0.000002, -0.001008, -0.000000, -0.000001, 0.000002, -0.000018, 0.001728, 0.000190, 0.001369, 0.000006, 
];
const FC1_BIAS: [f32; 8] = [-0.484898, 0.004889, -0.645173, -0.654344, 0.607789, -0.036724, 0.675956, -0.005114];
const FC2_WEIGHT: [f32; 8] = [-0.237678, 0.931518, -0.572972, -0.758378, 0.454089, -0.023281, 0.571801, 0.000068];
const FC2_BIAS: f32 = 0.4064;

const Y_MEAN: f32 = -2257.0457;
const Y_STD: f32 = 1822.3795;

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 13;

pub fn lmr_forward(mut input: [f32; INPUT_SIZE]) -> f32 {
    input[0] = (input[0] + 593.73) / 4011.14;
    input[1] /= 92.83;
    input[2] /= 9.53;
    input[3] /= 3.78;

    let mut hidden = [0.0; HIDDEN_SIZE];
    for i in 0..HIDDEN_SIZE {
        let mut output = 0.0;
        for j in 0..INPUT_SIZE {
            output += FC1_WEIGHT[i * INPUT_SIZE + j] * input[j];
        }
        hidden[i] = relu(output + FC1_BIAS[i]);
    }

    let mut y = 0.0;
    for i in 0..HIDDEN_SIZE {
        y += FC2_WEIGHT[i] * hidden[i];
    }
    y += FC2_BIAS;
    y * Y_STD + Y_MEAN
}
