const LEN: usize = 11;

const SINGLE_VALUES_LEN: usize = LEN;
const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;

const SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-695, -600, 1776, -82, -27, -88, 152, 48, 1388, -808, -911];
const DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
    99, -6, 123, 29, 14, 249, -511, -156, -43, 60, -712, -573, 44, -1006, 94, 81, 18, -40, -129, 57, -94, 10, -17, -33,
    -101, 19, 145, 42, -73, -80, 25, -8, 113, 87, 43, 835, 214, -146, 52, 7, -79, -47, -144, -69, 21, 170, -5, 66, -83,
    -168, -1, -131, -18, 77, 23,
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
