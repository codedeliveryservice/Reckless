pub const PIECE_VALUES: [i32; 7] = [100, 375, 400, 625, 1200, 0, 0];

pub fn lmp_threshold(depth: i32) -> i32 {
    3 + depth * depth
}
