use std::time::{Duration, Instant};

use crate::thread::ThreadData;

pub enum Limits {
    Infinite,
    Depth(i32),
    Time(u64),
    Nodes(u64),
    Fischer(u64, u64),
    Cyclic(u64, u64, u64),
}

const TIME_OVERHEAD_MS: u64 = 15;

pub struct TimeManager {
    limits: Limits,
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
}

impl TimeManager {
    pub fn new(limits: Limits) -> Self {
        let soft;
        let hard;

        match limits {
            Limits::Time(ms) => {
                soft = ms;
                hard = ms;
            }
            Limits::Fischer(main, inc) => {
                soft = (0.035 * (main as f64 + 0.75 * inc as f64)) as u64;
                hard = (0.135 * (main as f64 + 0.75 * inc as f64)) as u64;
            }
            Limits::Cyclic(main, inc, moves) => {
                let base = (main as f64 / moves as f64) + 0.75 * inc as f64;

                soft = ((1.0 * base) as u64).min(main + inc);
                hard = ((5.0 * base) as u64).min(main + inc);
            }
            _ => {
                soft = u64::MAX;
                hard = u64::MAX;
            }
        }

        Self {
            limits,
            start_time: Instant::now(),
            soft_bound: Duration::from_millis(soft.saturating_sub(TIME_OVERHEAD_MS)),
            hard_bound: Duration::from_millis(hard.saturating_sub(TIME_OVERHEAD_MS)),
        }
    }

    pub fn soft_limit(&self, depth: i32, nodes: u64) -> bool {
        match self.limits {
            Limits::Infinite => false,
            Limits::Depth(maximum) => depth >= maximum,
            Limits::Nodes(maximum) => nodes >= maximum,
            _ => self.start_time.elapsed() >= self.soft_bound,
        }
    }

    pub fn check_time(&self, td: &ThreadData) -> bool {
        if td.completed_depth == 0 {
            return false;
        }

        if td.nodes & 2047 == 2047 && td.get_stop() {
            return true;
        }

        match self.limits {
            Limits::Infinite | Limits::Depth(_) => false,
            Limits::Nodes(maximum) => td.nodes >= maximum,
            _ => td.nodes & 2047 == 2047 && self.start_time.elapsed() >= self.hard_bound,
        }
    }
}
