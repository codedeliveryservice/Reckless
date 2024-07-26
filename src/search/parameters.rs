pub const RFP_MARGIN: i32 = 75;
pub const RFP_DEPTH: i32 = 7;

pub const RAZORING_DEPTH: i32 = 4;
pub const RAZORING_MARGIN: i32 = 220;
pub const RAZORING_FIXED_MARGIN: i32 = 135;

pub const LMR_MOVES_PLAYED: i32 = 3;
pub const LMR_DEPTH: i32 = 3;
pub const LMR_BASE: f64 = 0.75;
pub const LMR_DIVISOR: f64 = 2.25;
pub const LMR_HISTORY_DIVISOR: f64 = 6200.0;

pub const FP_DEPTH: i32 = 5;
pub const FP_MARGIN: i32 = 130;
pub const FP_FIXED_MARGIN: i32 = 45;

pub const LMP_DEPTH: i32 = 4;
pub const LMP_MARGIN: i32 = 3;

pub const DEEPER_SEARCH_MARGIN: i32 = 80;
pub const IIR_DEPTH: i32 = 4;

pub const SEE_MARGIN: i32 = 100;
pub const SEE_DEPTH: i32 = 6;

pub struct Parameters {
    lmr: [[f64; 64]; 64],
}

impl Parameters {
    pub fn lmr(&self, depth: i32, moves_played: i32) -> f64 {
        self.lmr[depth.min(63) as usize][moves_played.min(63) as usize]
    }
}

impl Default for Parameters {
    fn default() -> Self {
        let mut lmr = [[0f64; 64]; 64];
        for depth in 0..64 {
            for moves in 0..64 {
                lmr[depth][moves] = LMR_BASE + (depth as f64).ln() * (moves as f64).ln() / LMR_DIVISOR;
            }
        }
        Self { lmr }
    }
}
