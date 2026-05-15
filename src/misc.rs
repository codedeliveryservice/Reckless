#![allow(dead_code)]

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicI64, Ordering},
};

const SLOTS: usize = 32;

pub static HITS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];
pub static STATS: [Wrapper; SLOTS] = [const { Wrapper::new() }; SLOTS];

pub struct Wrapper {
    data: [AtomicI64; 3],
    values: Mutex<Vec<i64>>,
    label: OnceLock<&'static str>,
}

impl Wrapper {
    const fn new() -> Self {
        Self {
            data: [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)],
            values: Mutex::new(Vec::new()),
            label: OnceLock::new(),
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
        if sorted.len().is_multiple_of(2) { (sorted[mid - 1] + sorted[mid]) as f64 / 2.0 } else { sorted[mid] as f64 }
    }

    fn gini_mean_difference(&self) -> f64 {
        let vals = self.values.lock().unwrap();
        let len = vals.len();
        if len < 2 {
            return 0.0;
        }

        let mut sorted = vals.to_vec();
        sorted.sort_unstable();
        let mut sum = 0.0;
        let mut total = 0.0;

        for (count, &value) in sorted.iter().enumerate() {
            let value = value as f64;
            let count = count as f64;

            total += count * value - sum;
            sum += value;
        }

        2.0 * total / (len as f64 * (len as f64 - 1.0))
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

pub fn slot_for(slots: &[Wrapper; SLOTS], label: &'static str) -> usize {
    if let Some((slot, _)) = slots.iter().enumerate().find(|(_, entry)| entry.label.get() == Some(&label)) {
        return slot;
    }

    for (slot, entry) in slots.iter().enumerate() {
        if entry.label.set(label).is_ok() || entry.label.get() == Some(&label) {
            return slot;
        }
    }

    panic!("debug slot limit exceeded");
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

fn grouped(n: i64) -> String {
    let sign = if n < 0 { "-" } else { "" };
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + sign.len());

    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('\'');
        }
        out.push(ch);
    }

    let mut out: String = out.chars().rev().collect();
    if !sign.is_empty() {
        out.insert(0, '-');
    }

    out
}

#[macro_export]
macro_rules! dbg_hit {
    ($condition:expr) => {
        $crate::misc::dbg_hit(
            $condition,
            $crate::misc::slot_for(
                &$crate::misc::HITS,
                concat!("[", file!(), ":", line!(), ":", column!(), "] (", stringify!($condition), ")"),
            ),
        )
    };
}

#[macro_export]
macro_rules! dbg_stats {
    ($value:expr) => {
        $crate::misc::dbg_stats(
            $value,
            $crate::misc::slot_for(
                &$crate::misc::STATS,
                concat!("[", file!(), ":", line!(), ":", column!(), "] (", stringify!($value), ")"),
            ),
        )
    };
}

pub fn dbg_print() {
    for slot in &HITS {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let hits = slot.get(1);
            let rate = hits as f64 / total as f64 * 100.0;
            let label = slot.label.get().copied().unwrap_or("<unknown>");
            println!("{label} {} / {} ({rate:.5}%)", grouped(hits), grouped(total));
        }
    }

    for slot in &STATS {
        if slot.get(0) > 0 {
            let total = slot.get(0);
            let mean = slot.get(1) as f64 / total as f64;
            let variance = slot.get(2) as f64 / total as f64 - mean * mean;
            let stddev = variance.sqrt();
            let median = slot.median();
            let gmd = slot.gini_mean_difference();
            let min = slot.min();
            let max = slot.max();
            let label = slot.label.get().copied().unwrap_or("<unknown>");
            println!(
                "{label} Total {}, Mean {mean:.5}, Median {median:.5}, SD {stddev:.5}, GMD {gmd:.5}, Min {min}, Max {max}",
                grouped(total)
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
