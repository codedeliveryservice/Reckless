use std::time::{Duration, Instant};

use crate::thread::ThreadData;

#[derive(Clone, Debug)]
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
    pub fn new(limits: Limits, move_overhead: u64) -> Self {
        let soft;
        let hard;

        match limits {
            Limits::Time(ms) => {
                soft = ms;
                hard = ms;
            }
            Limits::Fischer(main, inc) => {
                let moves_to_go = 50;
                let estimated_total_time = (main + moves_to_go * inc).saturating_sub(moves_to_go * move_overhead);

                let soft_scale = (0.051 * estimated_total_time as f64).min(0.443 * main as f64);
                let hard_scale = 0.742;

                let soft_bound = (soft_scale * main.saturating_sub(move_overhead) as f64 + 0.75 * inc as f64) as u64;
                let hard_bound = (hard_scale * main.saturating_sub(move_overhead) as f64 + 0.75 * inc as f64) as u64;

                soft = soft_bound.min(main.saturating_sub(move_overhead));
                hard = hard_bound.min(main.saturating_sub(move_overhead));
            }
            Limits::Cyclic(main, inc, moves) => {
                let main = main.saturating_sub(move_overhead);
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

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn soft_limit(&self, td: &ThreadData, multiplier: impl Fn() -> f32) -> bool {
        match self.limits {
            Limits::Infinite => false,
            Limits::Depth(maximum) => td.completed_depth >= maximum,
            Limits::Nodes(maximum) => td.nodes.global() >= maximum,
            Limits::Time(maximum) => self.start_time.elapsed() >= Duration::from_millis(maximum),
            _ => self.start_time.elapsed() >= Duration::from_secs_f32(self.soft_bound.as_secs_f32() * multiplier()),
        }
    }

    pub fn check_time(&self, td: &ThreadData) -> bool {
        if td.completed_depth == 0 {
            return false;
        }

        if td.nodes.local() & 2047 == 2047 && td.get_stop() {
            return true;
        }

        match self.limits {
            Limits::Infinite | Limits::Depth(_) => false,
            Limits::Nodes(maximum) => td.nodes.global() >= maximum,
            _ => td.nodes.local() & 2047 == 2047 && self.start_time.elapsed() >= self.hard_bound,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits.clone()
    }
}
