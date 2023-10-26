use std::time::{Duration, Instant};

use crate::types::MAX_SEARCH_DEPTH;

#[derive(Debug, PartialEq, Eq)]
pub enum Limits {
    Infinite,
    FixedTime(u64),
    FixedDepth(i32),
    Tournament(u64, u64, Option<u64>),
}

const TIME_OVERHEAD_MS: u64 = 20;
const MOVES_TO_GO: u64 = 25;

pub struct TimeManager {
    limits: Limits,
    start_time: Instant,
    allocated_time: Duration,
}

impl TimeManager {
    pub fn new(limits: Limits) -> Self {
        Self {
            allocated_time: Duration::from_millis(calculate_time_ms(&limits)),
            start_time: Instant::now(),
            limits,
        }
    }

    pub const fn get_max_depth(&self) -> i32 {
        match self.limits {
            Limits::FixedDepth(depth) => depth,
            _ => MAX_SEARCH_DEPTH,
        }
    }

    pub fn is_time_over(&self) -> bool {
        self.start_time.elapsed() >= self.allocated_time
    }
}

fn calculate_time_ms(limits: &Limits) -> u64 {
    match *limits {
        Limits::FixedTime(ms) => ms,
        Limits::Tournament(main, inc, moves) => {
            let moves_to_go = moves.unwrap_or(MOVES_TO_GO);
            (main / moves_to_go + inc).saturating_sub(TIME_OVERHEAD_MS)
        }
        _ => u64::MAX,
    }
}
