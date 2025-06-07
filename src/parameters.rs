pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

pub fn print_options() {
    for i in 0..unsafe { FC1_WEIGHT }.len() {
        println!("option name fc1_weight_{i} type string");
    }
    for i in 0..unsafe { FC1_BIAS }.len() {
        println!("option name fc1_bias_{i} type string");
    }
    for i in 0..unsafe { FC2_WEIGHT }.len() {
        println!("option name fc2_weight_{i} type string");
    }
    println!("option name fc2_bias type string");
    println!("option name y_mean type string");
    println!("option name y_std type string");
    println!("option name offset_0 type string");
    println!("option name scale_0 type string");
    println!("option name scale_1 type string");
    println!("option name scale_2 type string");
    println!("option name scale_3 type string");
}

pub fn print_parameters() {
    for i in 0..unsafe { FC1_WEIGHT }.len() {
        println!("fc1_weight_{i}, float, {}, -2.0, 2.0, 0.05, 0.002", unsafe { FC1_WEIGHT[i] });
    }
    for i in 0..unsafe { FC1_BIAS }.len() {
        println!("fc1_bias_{i}, float, {}, -2.0, 2.0, 0.05, 0.002", unsafe { FC1_BIAS[i] });
    }
    for i in 0..unsafe { FC2_WEIGHT }.len() {
        println!("fc2_weight_{i}, float, {}, -2.0, 2.0, 0.05, 0.002", unsafe { FC2_WEIGHT[i] });
    }
    println!("fc2_bias, float, {}, -1.0, 1.0, 0.05, 0.002", unsafe { FC2_BIAS });
    println!("y_mean, float, {}, -5000.0, 5000.0, 75, 0.002", unsafe { Y_MEAN });
    println!("y_std, float, {}, 0.0, 5000.0, 50, 0.002", unsafe { Y_STD });
    println!("offset_0, float, {}, 0.0, 1000.0, 30, 0.002", unsafe { OFFSET_0 });
    println!("scale_0, float, {}, 0.0, 10000.0, 50, 0.002", unsafe { SCALE_0 });
    println!("scale_1, float, {}, 0.0, 200.0, 4.641, 0.002", unsafe { SCALE_1 });
    println!("scale_2, float, {}, 0.0, 100.0, 0.476, 0.002", unsafe { SCALE_2 });
    println!("scale_3, float, {}, 0.0, 100.0, 0.189, 0.002", unsafe { SCALE_3 });
}

pub fn set_parameter(name: &str, value: &str) {
    if name.starts_with("fc1_weight_") {
        let index: usize = name[11..].parse().unwrap();
        unsafe { FC1_WEIGHT[index] = value.parse().unwrap() };
    } else if name.starts_with("fc1_bias_") {
        let index: usize = name[9..].parse().unwrap();
        unsafe { FC1_BIAS[index] = value.parse().unwrap() };
    } else if name.starts_with("fc2_weight_") {
        let index: usize = name[11..].parse().unwrap();
        unsafe { FC2_WEIGHT[index] = value.parse().unwrap() };
    } else if name == "fc2_bias" {
        unsafe { FC2_BIAS = value.parse().unwrap() };
    } else if name == "y_mean" {
        unsafe { Y_MEAN = value.parse().unwrap() };
    } else if name == "y_std" {
        unsafe { Y_STD = value.parse().unwrap() };
    } else if name == "offset_0" {
        unsafe { OFFSET_0 = value.parse().unwrap() };
    } else if name == "scale_0" {
        unsafe { SCALE_0 = value.parse().unwrap() };
    } else if name == "scale_1" {
        unsafe { SCALE_1 = value.parse().unwrap() };
    } else if name == "scale_2" {
        unsafe { SCALE_2 = value.parse().unwrap() };
    } else if name == "scale_3" {
        unsafe { SCALE_3 = value.parse().unwrap() };
    } else {
        panic!("Unknown parameter name: {name}");
    }
}

fn relu(x: f32) -> f32 {
    if x > 0.0 {
        x
    } else {
        0.0
    }
}

static mut FC1_WEIGHT: [f32; 104] = [
    0.007362, 0.168413, 0.274671, 0.137478, 0.380552, -0.003706, 0.051949, -0.122631, 0.180883, 0.102694, 0.197603,
    0.116325, 0.165675, -0.000543, -0.000288, -0.000378, -0.001142, 1.259012, -0.369488, -0.453419, -0.440387,
    -0.016561, 0.014386, -0.002813, -0.002147, -0.002135, 0.029055, 0.301576, 0.176181, 0.005513, 0.444576, 0.110555,
    0.205859, 0.020442, 0.170089, 0.155703, 0.352527, 0.301357, 0.397965, 0.070711, 0.249596, 0.259692, 0.006622,
    0.767045, 0.218360, 0.218852, -0.172881, 0.380728, 0.246253, 0.147029, 0.283069, 0.350570, -0.223930, 0.087688,
    0.208027, 0.143679, -0.734394, 0.314693, 0.283507, 0.496607, 0.021395, 0.134617, -0.048748, 0.093851, 0.022525,
    0.016505, 0.117419, 0.046631, -0.146664, 0.159141, -0.016512, -0.008243, -0.143072, 0.002406, -0.010326, -0.011541,
    -0.003682, -0.009147, -0.064574, 0.351799, -0.021491, 0.500016, -0.525388, 0.141220, 0.285497, 0.431342, -0.119255,
    0.130823, -0.115592, -0.114268, -0.000275, 0.006517, -0.028573, -0.027091, 0.000002, -0.001008, -0.000000,
    -0.000001, 0.000002, -0.000018, 0.001728, 0.000190, 0.001369, 0.000006,
];
static mut FC1_BIAS: [f32; 8] = [-0.484898, 0.004889, -0.645173, -0.654344, 0.607789, -0.036724, 0.675956, -0.005114];
static mut FC2_WEIGHT: [f32; 8] = [-0.237678, 0.931518, -0.572972, -0.758378, 0.454089, -0.023281, 0.571801, 0.000068];
static mut FC2_BIAS: f32 = 0.4064;

static mut Y_MEAN: f32 = -2257.0457;
static mut Y_STD: f32 = 1822.3795;

static mut OFFSET_0: f32 = 593.73;
static mut SCALE_0: f32 = 4011.14;
static mut SCALE_1: f32 = 92.83;
static mut SCALE_2: f32 = 9.53;
static mut SCALE_3: f32 = 3.78;

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 13;

pub unsafe fn lmr_forward(mut input: [f32; INPUT_SIZE]) -> f32 {
    input[0] = (input[0] + OFFSET_0) / SCALE_0;
    input[1] /= SCALE_1;
    input[2] /= SCALE_2;
    input[3] /= SCALE_3;

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
