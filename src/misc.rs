#![allow(dead_code)]

use std::sync::atomic::{AtomicI64, Ordering};

const SLOTS: usize = 32;

static HITS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];
static STATS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];
static BAYES: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];

struct Wrapper {
    data: [AtomicI64; 3],
}

impl Wrapper {
    const fn new() -> Self {
        Self {
            data: [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)],
        }
    }

    fn add(&self, index: usize, value: i64) {
        self.data[index].fetch_add(value, Ordering::Relaxed);
    }

    fn get(&self, index: usize) -> i64 {
        self.data[index].load(Ordering::Relaxed)
    }

    fn reset(&self) {
        for i in 0..self.data.len() {
            self.data[i].store(0, Ordering::Relaxed);
        }
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

/// Record a conditional event for P(event | given).
pub fn dbg_bayes(event: bool, given: bool, slot: usize) -> bool {
    assert!(slot < SLOTS);

    if given {
        BAYES[slot].add(0, 1); // count "given" occurrences (G)
        if event {
            BAYES[slot].add(1, 1); // count joint (G ∧ E)
        }
    }

    // Optional: track marginal event count if you ever want P(E)
    if event {
        BAYES[slot].add(2, 1); // count "event" occurrences (E)
    }

    event
}

pub fn dbg_print() {
    // HITS
    for (i, slot) in HITS.iter().enumerate() {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let hits = slot.get(1);
            let rate = hits as f64 / total as f64 * 100.0;

            println!("Hit #{i}: Total {total}, Hits {hits}, Hit Rate (%) {rate:.2}");
        }
    }

    // STATS
    for (i, slot) in STATS.iter().enumerate() {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let mean = slot.get(1) as f64 / total as f64;
            let var = slot.get(2) as f64 / total as f64 - mean * mean;
            let stddev = var.sqrt();

            println!("Stats #{i}: Total {total}, Mean {mean:.2}, Std Dev {stddev:.2}");
        }
    }

    // BAYES: P(event | given) = joint / given
    for (i, slot) in BAYES.iter().enumerate() {
        let given = slot.get(0); // G
        let joint = slot.get(1); // G ∧ E
        let event = slot.get(2); // E (marginal; optional)
        if given > 0 {
            let p = joint as f64 / given as f64 * 100.0;
            println!("Bayes #{i}: given {given}, joint {joint}, event {event}, P(event|given) (%) {p:.2}");
        }
    }

    // Reset all
    for slot in &HITS {
        slot.reset();
    }

    for slot in &STATS {
        slot.reset();
    }
    for slot in &BAYES {
        slot.reset();
    }
}
