#![allow(dead_code)]

use std::sync::{
    atomic::{AtomicI64, Ordering},
    Mutex,
};

const SLOTS: usize = 32;

static HITS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];
static STATS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];

struct Wrapper {
    data: [AtomicI64; 3],
    values: Mutex<Vec<i64>>,
}

impl Wrapper {
    const fn new() -> Self {
        Self {
            data: [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)],
            values: Mutex::new(Vec::new()),
        }
    }

    fn add(&self, index: usize, value: i64) {
        self.data[index].fetch_add(value, Ordering::Relaxed);

        if index == 1 {
            let mut vals = self.values.lock().unwrap();
            vals.push(value);
        }
    }

    fn get(&self, index: usize) -> i64 {
        self.data[index].load(Ordering::Relaxed)
    }

    fn median(&self) -> f64 {
        let vals = self.values.lock().unwrap();
        if vals.is_empty() {
            return 0.0;
        }
        let mut sorted = vals.to_vec();
        sorted.sort_unstable();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        } else {
            sorted[mid] as f64
        }
    }

    fn gini_mean_difference(&self) -> f64 {
        let vals = self.values.lock().unwrap();
        let n = vals.len();
        if n < 2 {
            return 0.0;
        }
        let mut sorted = vals.to_vec();
        sorted.sort_unstable();
        let mut sum = 0i64;
        for (i, &x) in sorted.iter().enumerate() {
            sum += x * (2 * (i as i64) + 1 - n as i64);
        }
        2.0 * sum as f64 / (n as f64 * (n as f64 - 1.0))
    }

    fn min(&self) -> i64 {
        let vals = self.values.lock().unwrap();
        *vals.iter().min().unwrap_or(&0)
    }

    fn max(&self) -> i64 {
        let vals = self.values.lock().unwrap();
        *vals.iter().max().unwrap_or(&0)
    }

    fn reset(&self) {
        for i in 0..self.data.len() {
            self.data[i].store(0, Ordering::Relaxed);
        }
        self.values.lock().unwrap().clear();
    }
}

pub fn dbg_hit(condition: bool, slot: usize) -> bool {
    assert!(slot < SLOTS);

    HITS[slot].add(0, 1);
    if condition {
        HITS[slot].add(1, 1);
    }
    condition
}

pub fn dbg_stats<T: Into<i64> + Copy>(value: T, slot: usize) -> T {
    assert!(slot < SLOTS);

    let v = value.into();
    STATS[slot].add(0, 1);
    STATS[slot].add(1, v);
    STATS[slot].add(2, v * v);

    value
}

pub fn dbg_print() {
    for (i, slot) in HITS.iter().enumerate() {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let hits = slot.get(1);
            let rate = hits as f64 / total as f64 * 100.0;

            println!("Hit #{i}: Total {total}, Hits {hits}, Hit Rate (%) {rate:.5}");
        }
    }

    for (i, slot) in STATS.iter().enumerate() {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let mean = slot.get(1) as f64 / total as f64;
            let variance = slot.get(2) as f64 / total as f64 - mean * mean;
            let stddev = variance.sqrt();
            let median = slot.median();
            let gmd = slot.gini_mean_difference();
            let min = slot.min();
            let max = slot.max();

            println!(
                "Stats #{i}: Total {total}, Mean {mean:.5}, Median {median:.5}, Std Dev {stddev:.5}, GMD {gmd:.5}, Min {min}, Max {max}"
            );
        }
    }

    for slot in &HITS {
        slot.reset();
    }

    for slot in &STATS {
        slot.reset();
    }
}
