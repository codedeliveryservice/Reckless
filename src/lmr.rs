const LEN: usize = 11;

pub const SINGLE_VALUES_LEN: usize = LEN;
pub const DOUBLE_VALUES_LEN: usize = LEN * (LEN - 1) / 2;

pub static mut SINGLE_VALUES: [i32; SINGLE_VALUES_LEN] = [-590, -663, 1193, 0, 0, 0, 0, 0, 1232, -794, -794];
pub static mut DOUBLE_VALUES: [i32; DOUBLE_VALUES_LEN] = [
    0, 0, 0, 0, 0, 0, -573, 0, 0, 0, -796, -652, 0, -783, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 768, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn feature_interaction(features: &[bool]) -> i32 {
    let mut output = 0;

    for i in 0..LEN {
        output += unsafe { SINGLE_VALUES[i] } * features[i] as i32;
    }

    let mut idx = 0;
    for i in 0..LEN {
        for j in (i + 1)..LEN {
            output += unsafe { DOUBLE_VALUES[idx] } * (features[i] && features[j]) as i32;
            idx += 1;
        }
    }

    output
}
