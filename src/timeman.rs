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

const NODE_TM_DEPTH_MARGIN: i32 = 10;
const NODE_TM_MULTIPLIER: f64 = 1.35;

const TIME_OVERHEAD_MS: u64 = 15;
const HARD_BOUND: u64 = 8;
const SOFT_BOUND: u64 = 40;

const INCREMENT_MULT: f64 = 0.75;

const TOURNAMENT_SOFT_MULT: f64 = 1.0;
const TOURNAMENT_HARD_MULT: f64 = 5.0;

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

    pub fn is_soft_bound_reached(&self, depth: i32, effort: f64) -> bool {
        let mut soft_bound = self.soft_bound.as_secs_f64();

        if depth >= NODE_TM_DEPTH_MARGIN {
            soft_bound *= (1.5 - effort) * NODE_TM_MULTIPLIER;
        }

        self.start_time.elapsed() >= Duration::from_secs_f64(soft_bound)
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
            let base = (main as f64 / moves as f64) + INCREMENT_MULT * inc as f64;

            let soft = (TOURNAMENT_SOFT_MULT * base) as u64;
            let hard = (TOURNAMENT_HARD_MULT * base) as u64;

            let soft = soft.min(main + inc).saturating_sub(TIME_OVERHEAD_MS);
            let hard = hard.min(main + inc).saturating_sub(TIME_OVERHEAD_MS);

            (soft, hard)
        }
        _ => (u64::MAX, u64::MAX),
    }
}
