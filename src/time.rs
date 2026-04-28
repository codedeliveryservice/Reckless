use std::time::{Duration, Instant};

use crate::thread::ThreadData;

#[derive(Clone, Debug)]
pub enum Limits {
    Infinite,
    Depth(i32),
    Time(u64),
    Nodes(u64),
    Clock { main: u64, inc: u64, moves_to_go: Option<u64> },
}

const TIME_OVERHEAD_MS: u64 = 15;

#[derive(Clone)]
pub struct TimeManager {
    limits: Limits,
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
}

impl TimeManager {
    pub fn new(limits: Limits, fullmove_number: usize, move_overhead: u64) -> Self {
        let soft;
        let hard;

        match limits {
            Limits::Time(ms) => {
                soft = ms;
                hard = ms;
            }
            Limits::Clock { main, inc, moves_to_go } => {
                let moves_to_go = moves_to_go.unwrap_or(50 - fullmove_number.max(40) as u64) as f32;

                let budget = main as f32 / moves_to_go + 0.500 * inc as f32 - move_overhead as f32;

                let soft_limit = 1.200 * budget;
                let hard_limit = 0.750 * main as f32;

                soft = (soft_limit as u64).min(main.saturating_sub(move_overhead));
                hard = (hard_limit as u64).min(main.saturating_sub(move_overhead));
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
            Limits::Infinite | Limits::Depth(_) => false,
            Limits::Nodes(maximum) => td.shared.nodes.aggregate() >= maximum,
            Limits::Time(maximum) => self.start_time.elapsed() >= Duration::from_millis(maximum),
            _ => self.start_time.elapsed() >= Duration::from_secs_f32(self.soft_bound.as_secs_f32() * multiplier()),
        }
    }

    pub fn check_time(&self, td: &ThreadData) -> bool {
        if td.completed_depth == 0 {
            return false;
        }

        match self.limits {
            Limits::Infinite | Limits::Depth(_) => false,
            Limits::Nodes(maximum) => td.shared.nodes.aggregate() > maximum,
            _ => td.nodes() & 2047 == 2047 && self.start_time.elapsed() >= self.hard_bound,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits.clone()
    }
}
