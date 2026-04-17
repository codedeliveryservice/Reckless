use std::sync::atomic::Ordering;
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

#[derive(Clone)]
pub struct TimeManager {
    limits: Limits,
    ponder: bool,
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::thread::SharedContext;

    use super::*;

    #[test]
    fn test_ponderhit_turns_off_pondering_in_timer() {
        let mut tm = TimeManager::new(Limits::Time(10_000), 0, 0, true);
        assert!(tm.is_ponder());
        tm.on_ponderhit();
        assert!(!tm.is_ponder());
    }

    #[test]
    fn test_ponderhit_preserves_elapsed_time() {
        let mut tm = TimeManager::new(Limits::Time(10_000), 0, 0, true);
        std::thread::sleep(Duration::from_millis(10));
        let before = tm.elapsed();
        tm.on_ponderhit();
        let after = tm.elapsed();
        assert!(after >= before);
    }

    #[test]
    fn test_soft_limit_ignored_while_pondering() {
        let shared = Arc::new(SharedContext::default());
        let mut td = crate::thread::ThreadData::new(shared.clone());
        td.time_manager = TimeManager::new(Limits::Time(1), 0, 0, true);
        td.completed_depth = 1;

        shared.pondering.store(true, Ordering::Release);
        std::thread::sleep(Duration::from_millis(20));
        assert!(!td.time_manager.soft_limit(&td, || 1.0));

        shared.pondering.store(false, Ordering::Release);
        assert!(td.time_manager.soft_limit(&td, || 1.0));
    }
}

impl TimeManager {
    pub fn new(limits: Limits, fullmove_number: usize, move_overhead: u64, ponder: bool) -> Self {
        let soft;
        let hard;

        match limits {
            Limits::Time(ms) => {
                soft = ms;
                hard = ms;
            }
            Limits::Fischer(main, inc) => {
                let soft_scale = 0.066 - 0.042 * (-0.045 * fullmove_number as f64).exp();
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
            ponder,
            start_time: Instant::now(),
            soft_bound: Duration::from_millis(soft.saturating_sub(TIME_OVERHEAD_MS)),
            hard_bound: Duration::from_millis(hard.saturating_sub(TIME_OVERHEAD_MS)),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn soft_limit(&self, td: &ThreadData, multiplier: impl Fn() -> f32) -> bool {
        if self.ponder && td.shared.pondering.load(Ordering::Acquire) {
            return false;
        }

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

        if self.ponder && td.shared.pondering.load(Ordering::Acquire) {
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

    pub fn is_ponder(&self) -> bool {
        self.ponder
    }

    pub fn on_ponderhit(&mut self) {
        self.ponder = false;
    }
}
