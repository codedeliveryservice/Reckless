const LEN: usize = 11;

const SINGLE_VALUES_LEN: usize = LEN;
const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;

const SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-744, -608, 1717, -92, 121, -94, 236, -28, 1417, -767, -915];
const DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
    41, -90, 40, 24, -19, 199, -489, -169, -54, 57, -727, -546, 48, -971, 57, 18, 11, -25, -156, 3, -139, -3, -25, -70,
    -33, 9, 132, 109, -44, -50, -6, 37, 125, 83, 92, 886, 246, -104, 62, -30, -109, -3, -174, -87, 51, 118, 15, 52,
    -60, -117, 44, -100, 26, 66, 34,
];

pub fn feature_interaction(features: &[bool]) -> i32 {
    let mut output = 0;

    for i in 0..LEN {
        output += SINGLE_VALUES[i] * features[i] as i32;
    }

    let mut idx = 0;
    for i in 0..LEN {
        for j in (i + 1)..LEN {
            output += DOUBLE_VALUES[idx] * (features[i] && features[j]) as i32;
            idx += 1;
        }
    }

    output
}
