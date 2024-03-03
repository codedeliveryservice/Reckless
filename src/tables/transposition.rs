use crate::types::{Move, Score};

pub const DEFAULT_TT_SIZE: usize = 16;

const MEGABYTE: usize = 1024 * 1024;
const INTERNAL_ENTRY_SIZE: usize = std::mem::size_of::<InternalEntry>();

#[allow(clippy::assertions_on_constants)]
const _: () = assert!(INTERNAL_ENTRY_SIZE == 8, "InternalEntry size is not 8 bytes");

#[derive(Copy, Clone)]
pub struct Entry {
    pub depth: i32,
    pub score: i32,
    pub bound: Bound,
    pub mv: Move,
}

/// Type of the score returned by the search.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

/// Internal representation of a transposition table entry (8 bytes).
#[derive(Copy, Clone)]
struct InternalEntry {
    key: u16,     // 2 bytes
    depth: u8,    // 1 byte
    score: i16,   // 2 bytes
    bound: Bound, // 1 byte
    mv: Move,     // 2 bytes
}

/// The transposition table is used to cache previously performed search results.
pub struct TranspositionTable {
    vector: Vec<Option<InternalEntry>>,
}

impl TranspositionTable {
    /// Creates a new transposition table with a total allocated size in megabytes.
    pub fn new(megabytes: usize) -> Self {
        Self {
            vector: vec![None; megabytes * MEGABYTE / INTERNAL_ENTRY_SIZE],
        }
    }

    /// Sets all entries to `None` without affecting the allocated memory or vector length.
    pub fn clear(&mut self) {
        self.vector.iter_mut().for_each(|entry| *entry = None);
    }

    /// Resizes the transposition table to the specified size in megabytes. This will clear all entries.
    pub fn resize(&mut self, megabytes: usize) {
        self.vector = vec![None; megabytes * MEGABYTE / INTERNAL_ENTRY_SIZE];
        println!("info string set Hash to {megabytes} MB");
    }

    /// Returns the approximate load factor of the transposition table in permille (on a scale of `0` to `1000`).
    pub fn get_load_factor(&self) -> usize {
        const BATCH_SIZE: usize = 10_000;
        self.vector.iter().take(BATCH_SIZE).filter(|slot| slot.is_some()).count() * 1000 / BATCH_SIZE
    }

    /// Reads an entry from the transposition table.
    pub fn read(&self, hash: u64, ply: usize) -> Option<Entry> {
        let index = self.get_index(hash);
        let entry = self.vector[index]?;

        if entry.key != verification_key(hash) {
            return None;
        }

        let mut hit = Entry {
            depth: i32::from(entry.depth),
            score: i32::from(entry.score),
            bound: entry.bound,
            mv: entry.mv,
        };

        // Adjust mate distance from "plies from the current position" to "plies from the root"
        if hit.score.abs() > Score::MATE_BOUND {
            hit.score -= hit.score.signum() * ply as i32;
        }
        Some(hit)
    }

    /// Writes an entry to the transposition table overwriting an existing one.
    pub fn write(&mut self, hash: u64, depth: i32, mut score: i32, bound: Bound, mut mv: Move, ply: usize) {
        // Adjust mate distance from "plies from the root" to "plies from the current position"
        if score.abs() > Score::MATE_BOUND {
            score += score.signum() * ply as i32;
        }

        let key = verification_key(hash);
        let index = self.get_index(hash);

        // Preserve the previous move if the new one is sourced from an upper bound node
        if let Some(old) = self.vector[index] {
            if bound == Bound::Upper && old.key == key && old.mv != Move::NULL {
                mv = old.mv;
            }
        }

        self.vector[index] = Some(InternalEntry {
            key,
            depth: depth as u8,
            score: score as i16,
            bound,
            mv,
        });
    }

    /// Prefetches the entry in the transposition table.
    pub fn prefetch(&self, hash: u64) {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

        #[cfg(target_arch = "x86_64")]
        unsafe {
            let index = self.get_index(hash);
            let ptr = self.vector.as_ptr().add(index).cast();
            _mm_prefetch::<_MM_HINT_T0>(ptr);
        }
    }

    /// Returns the index of the entry in the transposition table.
    fn get_index(&self, hash: u64) -> usize {
        // Fast hash table index calculation
        // For details, see: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
        let len = self.vector.len() as u128;
        ((u128::from(hash) * len) >> 64) as usize
    }
}

/// Returns the verification key of the hash (bottom 16 bits).
fn verification_key(hash: u64) -> u16 {
    hash as u16
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(DEFAULT_TT_SIZE)
    }
}