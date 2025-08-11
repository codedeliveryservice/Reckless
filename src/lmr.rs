const LEN: usize = 11;

const SINGLE_VALUES_LEN: usize = LEN;
const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;
const TRIPLE_VALUES_LEN: usize = LEN * (LEN - 1) * (LEN - 2) / 6;

const SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-706, -580, 1402, 91, 188, -267, -21, -275, 1047, -748, -683];
const DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
    135, -258, -32, -20, -39, 40, -651, 69, -75, -5, -872, -668, 166, -877, 95, -26, 0, -7, 127, 102, 135, 54, 56, -28,
    -86, -77, 94, 102, -185, 40, -162, 60, -62, -34, 5, 900, 9, 1, -31, 69, -98, -165, 150, -13, -8, 35, 139, -86, -11,
    -10, -106, -9, -32, -78, -203,
];
const TRIPLE_VALUES: [i32; TRIPLE_VALUES_LEN] = [
    -17, 22, 47, -2, 39, -12, 8, 28, 3, -11, -8, -16, -18, -11, -2, 6, 28, -29, -20, 2, 5, 17, -21, 0, -46, -42, 1, 31,
    48, 24, -45, 8, 9, 34, -11, -12, -1, -11, -21, 9, -20, -1, 4, -14, 10, -3, 12, -8, 17, -28, 0, -1, -3, -18, -8,
    -29, 7, 37, -11, 7, 4, -12, -11, -15, 14, -54, -19, -25, 2, 21, 51, 2, -4, 7, -30, -16, -36, 48, -15, 15, -34, 1,
    36, 34, -46, -9, -17, -14, 27, -11, -5, 13, -22, -5, 37, -26, -4, 30, -1, -12, 28, -4, 43, -1, 25, -1, -5, -1, -23,
    -6, 38, 29, 1, -36, 41, 29, -36, -35, -17, 6, -21, 17, 8, 18, 23, 27, -33, 7, 17, 1, -13, -8, 8, 10, -10, 4, 30,
    -32, 9, -5, -28, 11, -9, 42, 23, -7, 18, -7, 9, 25, 5, 10, -56, -52, -13, -26, -10, 13, -21, 7, -21, 28, -30, -20,
    -1,
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
