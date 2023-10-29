use std::time::{Duration, Instant};

use crate::types::MAX_SEARCH_DEPTH;

#[derive(Debug, PartialEq, Eq)]
pub enum Limits {
    Infinite,
    FixedTime(u64),
    FixedDepth(i32),
    Incremental(u64, u64),
    Tournament(u64, u64, u64),
}

const TIME_OVERHEAD_MS: u64 = 15;
const HARD_BOUND: u64 = 8;
const SOFT_BOUND: u64 = 40;

pub struct TimeManager {
    limits: Limits,
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
}

impl TimeManager {
    pub fn new(limits: Limits) -> Self {
        let (soft, hard) = calculate_time_ms(&limits);
        Self {
            start_time: Instant::now(),
            soft_bound: Duration::from_millis(soft),
            hard_bound: Duration::from_millis(hard),
            limits,
        }
    }

    pub const fn get_max_depth(&self) -> i32 {
        match self.limits {
            Limits::FixedDepth(depth) => depth,
            _ => MAX_SEARCH_DEPTH,
        }
    }

    pub fn is_soft_bound_reached(&self) -> bool {
        self.start_time.elapsed() >= self.soft_bound
    }

    pub fn is_hard_bound_reached(&self) -> bool {
        self.start_time.elapsed() >= self.hard_bound
    }
}

fn calculate_time_ms(limits: &Limits) -> (u64, u64) {
    match *limits {
        Limits::FixedTime(ms) => (ms, ms),
        Limits::Incremental(main, inc) => {
            let time = (main + inc).saturating_sub(TIME_OVERHEAD_MS);
            (time / SOFT_BOUND, time / HARD_BOUND)
        }
        Limits::Tournament(main, inc, moves) => {
            let time = (main / moves + inc).saturating_sub(TIME_OVERHEAD_MS);
            (time, time)
        }
        _ => (u64::MAX, u64::MAX),
    }
}
