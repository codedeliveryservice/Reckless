use crate::types::{Move, Score};

pub const MIN_CACHE_SIZE: usize = 1;
pub const MAX_CACHE_SIZE: usize = 512;
pub const DEFAULT_CACHE_SIZE: usize = 16;

const MEGABYTE: usize = 1024 * 1024;
const CACHE_ENTRY_SIZE: usize = std::mem::size_of::<Entry>();

#[allow(clippy::assertions_on_constants)]
const _: () = assert!(CACHE_ENTRY_SIZE == 8, "CacheEntry size is not 8 bytes");

/// A `CacheHit` is returned when a `Cache` entry is found.
#[derive(Copy, Clone)]
pub struct CacheHit {
    pub depth: i32,
    pub score: i32,
    pub bound: Bound,
    pub mv: Move,
}

/// A `Bound` is used to indicate the type of the score returned by the search.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

/// Internal representation of a `Cache` entry (8 bytes).
#[derive(Copy, Clone)]
struct Entry {
    key: u16,     // 2 bytes
    depth: u8,    // 1 byte
    score: i16,   // 2 bytes
    bound: Bound, // 1 byte
    mv: Move,     // 2 bytes
}

/// The transposition hash table is used to cache previously performed search results.
pub struct Cache {
    vector: Vec<Option<Entry>>,
}

impl Cache {
    /// Creates a new `Cache` with a total allocated size in megabytes.
    pub fn new(megabytes: usize) -> Self {
        Self {
            vector: vec![None; megabytes * MEGABYTE / CACHE_ENTRY_SIZE],
        }
    }

    /// Sets all entries to `None` without affecting the allocated memory or vector length.
    pub fn clear(&mut self) {
        self.vector.iter_mut().for_each(|entry| *entry = None);
    }

    /// Returns the approximate load factor of the `Cache` in permille (on a scale of `0` to `1000`).
    pub fn get_load_factor(&self) -> usize {
        const BATCH_SIZE: usize = 10_000;
        self.vector.iter().take(BATCH_SIZE).filter(|slot| slot.is_some()).count() * 1000 / BATCH_SIZE
    }

    /// Returns the `CacheHit` if the entry is found.
    pub fn read(&self, hash: u64, ply: usize) -> Option<CacheHit> {
        let index = self.get_index(hash);
        let entry = self.vector[index]?;

        if entry.key != verification_key(hash) {
            return None;
        }

        let mut hit = CacheHit {
            depth: i32::from(entry.depth),
            score: i32::from(entry.score),
            bound: entry.bound,
            mv: entry.mv,
        };

        if hit.score.abs() > Score::CHECKMATE_BOUND {
            hit.score -= hit.score.signum() * ply as i32;
        }
        Some(hit)
    }

    /// Writes an entry to the `Cache` overwriting an existing one.
    pub fn write(&mut self, hash: u64, depth: i32, mut score: i32, bound: Bound, mut mv: Move, ply: usize) {
        if score.abs() > Score::CHECKMATE_BOUND {
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

        self.vector[index] = Some(Entry {
            key,
            depth: depth as u8,
            score: score as i16,
            bound,
            mv,
        });
    }

    /// Returns the index of the entry in the `Cache` vector.
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

impl Default for Cache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }
}
