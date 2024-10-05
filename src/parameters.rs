pub const SEE_PIECE_VALUES: [i32; 7] = [100, 400, 400, 650, 1200, 0, 0];
pub const OPT_PIECE_VALUES: [i32; 7] = [400, 750, 800, 1200, 1900, 0, 0];

pub const RFP_MARGIN: i32 = 75;
pub const RFP_DEPTH: i32 = 7;

pub const RAZORING_DEPTH: i32 = 4;
pub const RAZORING_MARGIN: i32 = 220;
pub const RAZORING_FIXED_MARGIN: i32 = 135;

pub const LMR_MOVES_PLAYED: i32 = 3;
pub const LMR_DEPTH: i32 = 3;
pub const LMR_BASE: f64 = 0.73;
pub const LMR_DIVISOR: f64 = 2.22;
pub const LMR_HISTORY_DIVISOR: f64 = 6210.0;

pub const FP_DEPTH: i32 = 5;
pub const FP_MARGIN: i32 = 130;
pub const FP_FIXED_MARGIN: i32 = 45;

pub const LMP_DEPTH: i32 = 4;
pub const LMP_MARGIN: i32 = 3;

pub const DEEPER_SEARCH_MARGIN: i32 = 80;
pub const IIR_DEPTH: i32 = 4;

pub const SEE_NOISY_MARGIN: i32 = 100;
pub const SEE_QUIET_MARGIN: i32 = 70;
pub const SEE_DEPTH: i32 = 6;

pub struct Parameters {
    lmr: [[f64; 64]; 64],
}

impl Parameters {
    pub fn lmr(&self, depth: i32, moves: i32) -> f64 {
        self.lmr[depth.min(63) as usize][moves.min(63) as usize]
    }
}

impl Default for Parameters {
    fn default() -> Self {
        let mut lmr = [[0f64; 64]; 64];
        for (depth, row) in lmr.iter_mut().enumerate() {
            for (moves, r) in row.iter_mut().enumerate() {
                *r = LMR_BASE + (depth as f64).ln() * (moves as f64).ln() / LMR_DIVISOR;
            }
        }
        Self { lmr }
    }
}
