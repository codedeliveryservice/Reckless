use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::types::{Move, MAX_PLY};

#[derive(Debug, PartialEq, Eq)]
pub enum Limits {
    Infinite,
    FixedDepth(i32),
    FixedTime(u64),
    FixedNodes(u64),
    Fischer(u64, u64),
    Cyclic(u64, u64, u64),
}

const TIME_OVERHEAD_MS: u64 = 15;

const MAX_DEPTH: i32 = MAX_PLY as i32;
const MIN_NODES: u64 = 1024;

const UPDATE_DEPTH_MARGIN: i32 = 10;

const INCREMENT_MULT: f64 = 0.75;

const FISCHER_SOFT_MULT: f64 = 0.035;
const FISCHER_HARD_MULT: f64 = 0.135;

const CYCLIC_SOFT_MULT: f64 = 1.0;
const CYCLIC_HARD_MULT: f64 = 5.0;

pub struct TimeManager {
    abort_signal: &'static AtomicBool,
    start_time: Instant,
    soft_bound: Duration,
    hard_bound: Duration,
    max_depth: i32,
    max_nodes: u64,
    stability: usize,
    last_best_move: Option<Move>,
}

impl TimeManager {
    pub fn new(abort_signal: &'static AtomicBool, limits: Limits) -> Self {
        let (soft, hard) = calculates_bounds(&limits);

        Self {
            abort_signal,
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
            stability: 1,
            last_best_move: None,
        }
    }

    /// Informs the time manager that a search iteration has finished.
    pub fn update(&mut self, depth: i32, best_move: Move) {
        // The results of the first few iterations are not reliable
        if depth < UPDATE_DEPTH_MARGIN {
            return;
        }

        if self.last_best_move == Some(best_move) {
            self.stability += 1;
        } else {
            self.last_best_move = Some(best_move);
            self.stability = 1;
        }
    }

    /// Checks if the search should be stopped due to reaching the maximum depth or time.
    ///
    /// This method is used as a soft limit at the end of an iteration of iterative deepening.
    pub fn if_finished(&self, depth: i32, effort: f64) -> bool {
        if depth >= self.max_depth {
            return true;
        }

        let mut soft_bound = self.soft_bound.as_secs_f64();

        if depth >= UPDATE_DEPTH_MARGIN {
            // Adjust based on distribution of root nodes
            soft_bound *= 2.025 - 1.35 * effort;

            // Adjust based on stability of the best move between iterations
            soft_bound *= 0.75 + 0.75 / self.stability.min(7) as f64;
        }

        self.start_time.elapsed() >= Duration::from_secs_f64(soft_bound)
    }

    /// Checks if the maximum allocated time or nodes have been reached.
    pub fn is_time_up(&self, nodes: u64) -> bool {
        if nodes >= self.max_nodes {
            return true;
        }

        // Avoid pulling the timer too often to reduce the system call overhead
        if nodes & 2047 != 2047 {
            return false;
        }

        self.start_time.elapsed() >= self.hard_bound || self.abort_signal.load(Ordering::Relaxed)
    }
}

fn calculates_bounds(limits: &Limits) -> (u64, u64) {
    match *limits {
        Limits::FixedTime(ms) => (ms, ms),
        Limits::Fischer(main, inc) => {
            let soft = FISCHER_SOFT_MULT * (main as f64 + INCREMENT_MULT * inc as f64);
            let hard = FISCHER_HARD_MULT * (main as f64 + INCREMENT_MULT * inc as f64);

            let soft = (soft as u64).saturating_sub(TIME_OVERHEAD_MS);
            let hard = (hard as u64).saturating_sub(TIME_OVERHEAD_MS);

            (soft, hard)
        }
        Limits::Cyclic(main, inc, moves) => {
            let base = (main as f64 / moves as f64) + INCREMENT_MULT * inc as f64;

            let soft = (CYCLIC_SOFT_MULT * base) as u64;
            let hard = (CYCLIC_HARD_MULT * base) as u64;

            let soft = soft.min(main + inc).saturating_sub(TIME_OVERHEAD_MS);
            let hard = hard.min(main + inc).saturating_sub(TIME_OVERHEAD_MS);

            (soft, hard)
        }
        _ => (u64::MAX, u64::MAX),
    }
}
