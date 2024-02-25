use std::time::{Duration, Instant};

use crate::types::MAX_PLY;

#[derive(Debug, PartialEq, Eq)]
pub enum Limits {
    Infinite,
    FixedDepth(i32),
    FixedTime(u64),
    FixedNodes(u64),
    Incremental(u64, u64),
    Tournament(u64, u64, u64),
}

const NODE_TM_ADJUSTMENT: f64 = 1.35;

const TIME_OVERHEAD_MS: u64 = 15;
const HARD_BOUND: u64 = 8;
const SOFT_BOUND: u64 = 40;

const MAX_DEPTH: i32 = MAX_PLY as i32;
const MIN_NODES: u64 = 1024;

pub struct TimeManager {
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
    max_depth: i32,
    max_nodes: u64,
}

impl TimeManager {
    pub fn new(limits: Limits) -> Self {
        let (soft, hard) = calculate_time_ms(&limits);
        Self {
            start_time: Instant::now(),
            soft_bound: Duration::from_millis(soft),
            hard_bound: Duration::from_millis(hard),
            max_depth: match limits {
                Limits::FixedDepth(depth) => depth.min(MAX_DEPTH),
                _ => MAX_DEPTH,
            },
            max_nodes: match limits {
                Limits::FixedNodes(nodes) => nodes.max(MIN_NODES),
                _ => u64::MAX,
            },
        }
    }

    pub const fn max_depth(&self) -> i32 {
        self.max_depth
    }

    pub const fn max_nodes(&self) -> u64 {
        self.max_nodes
    }

    pub fn adjust(&mut self, effort: f64) {
        let adjustment = (1.5 - effort) * NODE_TM_ADJUSTMENT;
        let soft_bound = (self.soft_bound.as_millis() as f64 * adjustment) as u64;

        self.soft_bound = Duration::from_millis(soft_bound);
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
