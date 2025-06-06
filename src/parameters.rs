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
    0.0067, -0.1554, -0.1961, -0.0902, 0.1043, -0.0274, 0.1023, -0.2723, -0.0667, 0.0772, -0.0869, -0.1957, -0.2603,
    -0.3814, 0.0508, -0.0870, 0.2997, 0.0425, -0.1780, 0.3099, -0.0376, 0.2634, -0.1156, 0.0592, 0.3304, 0.0779,
    0.0175, -0.0058, 0.0097, 0.5325, -0.1401, -0.1921, -0.1072, -0.0652, 0.0724, 0.0086, -0.0247, -0.0233, -0.0321,
    0.0078, -0.0116, -0.1939, 0.3239, -0.1241, -0.1045, -0.0988, -0.4788, 0.1876, -0.0324, -0.0644, -0.0168, -0.1105,
    -0.0225, 0.1880, 0.1707, 0.4330, 0.3262, 0.2016, -0.1092, 0.2018, 0.1871, 0.3280, 0.2527, 0.0915, 0.3896, -0.2108,
    0.1496, 0.2766, -0.4849, 0.1867, 0.3226, 0.1092, 0.5099, -0.5231, -0.0360, -0.1219, -0.0415, -0.0446, 0.0523,
    -0.3212, 0.0042, 0.0094, -0.3045, -0.0217, -0.0865, -0.0731, -0.0666, 0.0283, -0.1747, -0.1306, 0.0474, -0.0138,
    0.1639, 0.1157, 0.3584, 0.1096, 0.3024, -0.1374, 0.2391, 0.2624, 0.2464, 0.3828, 0.1179, 0.3540,
];
const FC1_BIAS: [f32; 8] = [-0.0421, 0.8294, -0.1664, 1.3451, -0.6956, 0.6941, -0.0234, -0.5786];
const FC2_WEIGHT: [f32; 8] = [0.1459, 0.5427, 0.2208, 0.8604, -0.8111, 0.3485, -0.1330, -0.6048];
const FC2_BIAS: f32 = 0.3387;

const Y_MEAN: f32 = -3771.5005;
const Y_STD: f32 = 1588.2899;

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
