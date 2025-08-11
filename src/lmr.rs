const LEN: usize = 11;

const SINGLE_VALUES_LEN: usize = LEN;
const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;
const TRIPLE_VALUES_LEN: usize = LEN * (LEN - 1) * (LEN - 2) / 6;

const SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-590, -663, 1193, 0, 0, 0, 0, 0, 1232, -794, -794];
const DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
    0, 0, 0, 0, 0, 0, -573, 0, 0, 0, -796, -652, 0, -783, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 768, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const TRIPLE_VALUES: [i32; TRIPLE_VALUES_LEN] = [0; TRIPLE_VALUES_LEN];

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

    let mut idx = 0;
    for i in 0..LEN {
        for j in (i + 1)..LEN {
            for k in (j + 1)..LEN {
                output += TRIPLE_VALUES[idx] * (features[i] && features[j] && features[k]) as i32;
                idx += 1;
            }
        }
    }

    output
}
