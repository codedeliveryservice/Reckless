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
}

pub fn print_parameters() {
    for i in 0..unsafe { FC1_WEIGHT }.len() {
        println!("fc1_weight_{i}, float, {}, -1.0, 1.0, 0.1, 0.002", unsafe { FC1_WEIGHT[i] });
    }
    for i in 0..unsafe { FC1_BIAS }.len() {
        println!("fc1_bias_{i}, float, {}, -2.0, 2.0, 0.1, 0.002", unsafe { FC1_BIAS[i] });
    }
    for i in 0..unsafe { FC2_WEIGHT }.len() {
        println!("fc2_weight_{i}, float, {}, -1.0, 1.0, 0.1, 0.002", unsafe { FC2_WEIGHT[i] });
    }
    println!("fc2_bias, float, {}, -1.0, 1.0, 0.1, 0.002", unsafe { FC2_BIAS });
    println!("y_mean, float, {}, -5000.0, 5000.0, 150, 0.002", unsafe { Y_MEAN });
    println!("y_std, float, {}, 0.0, 5000.0, 75, 0.002", unsafe { Y_STD });
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
    } else {
        panic!("Unknown parameter name: {}", name);
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
    0.07677159944778501, -0.12346396700767352, -0.22928623432987238, -0.10394406966934315,
    0.10161783999364683, 0.0030363270827712006, 0.20037003859297753, -0.29573478963327277,
    0.0017954722408396447, -0.003758354636152976, -0.012326086271198041, -0.03501566019542505,
    -0.1702316430677795, -0.332490310624663, 0.027484854809147176, -0.1601056738292917,
    0.3819809698085826, 0.0052565949045553, -0.10667283444841509, 0.15726379154353537,
    0.10689947845115137, 0.2604029161034839, -0.24633316803358685, 0.027118181465629068,
    0.42152907282452573, 0.11679394542055742, 0.08101435624762239, 0.12216124911141836,
    0.08812944177740896, 0.4963043443426733, -0.19646987183873826, -0.1392842560162873,
    -0.1782326673620781, 0.05435024362859873, 0.171310044639422, 0.023562833639620605,
    0.11469167454950173, -0.1607895894123662, 0.19344135490266712, 0.03153192225173374,
    0.011776655543052373, -0.25926832406123973, 0.49195708476240063, -0.1086164640613843,
    -0.019904110507199774, -0.13867516732803986, -0.3898209810569392, 0.19171901098209176,
    -0.042901093768475125, -0.10223877170160788, -0.05331174983160734, -0.2390701621868936,
    0.02423161649909334, 0.04093198878217622, 0.2391488308877427, 0.2637125786568535,
    0.45045678417363527, 0.23656712463774937, -0.009318241414715272, 0.05218065643972957,
    0.2718560617309632, 0.3729350714153907, 0.2100806907621292, 0.13632089228875893,
    0.3881093327377603, -0.30689553665584096, 0.20040266302739795, 0.45910955821438465,
    -0.5373395453620782, 0.258133581958746, 0.22041183398576475, 0.18027871371187038,
    0.6311111343686406, -0.3657563176855129, 0.0027932828650807286, -0.22454736720496055,
    -0.029550360295146686, -0.006385984366061679, 0.034798875375966314, -0.21149479849294295,
    -0.008111885775167313, 0.05968796080126707, -0.2724607360808356, -0.1120632856443471,
    0.022007261593310932, 0.014078046309908743, -0.08212481729681649, 0.04303334994958088,
    -0.2322144076307931, -0.1842939842319084, 0.08989085605869689, 0.08114585471443465,
    0.2556981191731116, 0.31321164266871077, 0.3092101971225158, 0.25124534934808984,
    0.35762273457201893, -0.04995257044467546, 0.05486702864237975, 0.34295418631362123,
    0.2515291547130471, 0.41692477555998303, 0.24260904840659453, 0.3128874742495659
];

static mut FC1_BIAS: [f32; 8] =  [
    -0.008732825034456796, 1.0122124637400898, -0.0818397704967141, 1.4798094705030718,
    -0.6725046697483243, 0.5726373254996573, -0.15959632288167253, -0.7582277030373772
];
static mut FC2_WEIGHT: [f32; 8] =  [
    0.15203015908639966, 0.5936368284804547, 0.23043371246591604, 0.8179391199497501,
    -0.8676305066447771, 0.3931342300931851, -0.08885187341398014, -0.5740053497384392
];
static mut FC2_BIAS: f32 = 0.5595348799700917;

static mut Y_MEAN: f32 = -3755.8537806447152;
static mut Y_STD: f32 = 1507.5395104246325;

const HIDDEN_SIZE: usize = 8;
const INPUT_SIZE: usize = 13;

pub fn lmr_forward(input: [f32; INPUT_SIZE]) -> f32 {
    unsafe { forward(input) }
}

unsafe fn forward(mut input: [f32; 13]) -> f32 {
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
