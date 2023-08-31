use game::{Move, Score, Zobrist};

pub const DEFAULT_CACHE_SIZE: usize = 16;
pub const MAX_CACHE_SIZE: usize = 512;
pub const MIN_CACHE_SIZE: usize = 1;

/// The transposition hash table is used to cache previously performed search results.
pub struct Cache {
    vector: Vec<Option<CacheEntry>>,
}

impl Cache {
    /// Creates a new `Cache<T>` with a total allocated size in megabytes.
    pub fn new(megabytes: usize) -> Self {
        let length = megabytes * 1024 * 1024 / std::mem::size_of::<CacheEntry>();
        Self {
            vector: vec![Default::default(); length],
        }
    }

    /// Sets all entries to `None` without affecting the allocated memory or vector length.
    pub fn clear(&mut self) {
        self.vector.iter_mut().for_each(|entry| *entry = None);
    }

    /// Returns the approximate load factor of the `Cache` on a scale of `[0, 1000]`
    /// where `0` means empty and `1000` means 100% full.
    pub fn get_load_factor(&self) -> usize {
        const BATCH_SIZE: usize = 10_000;
        let occupied_slots = self.vector.iter().take(BATCH_SIZE).filter(|slot| slot.is_some()).count();
        occupied_slots * 1000 / BATCH_SIZE
    }

    /// Returns `Some(T)` if the entry was found; otherwise `None`.
    pub fn read(&self, hash: Zobrist, ply: usize) -> Option<CacheEntry> {
        let index = self.get_index(hash);
        let mut entry = self.vector[index]?;
        if entry.hash == hash {
            entry.adjust_mating_score(-(ply as i32));
            return Some(entry);
        }
        None
    }

    /// Writes an entry to the `Cache` overwriting an existing one.
    pub fn write(&mut self, mut entry: CacheEntry, ply: usize) {
        entry.adjust_mating_score(ply as i32);
        let index = self.get_index(entry.hash);
        self.vector[index] = Some(entry);
    }

    /// Returns the index of the entry in the `Cache` vector.
    fn get_index(&self, hash: Zobrist) -> usize {
        hash.0 as usize % self.vector.len()
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_SIZE)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Copy, Clone)]
pub struct CacheEntry {
    pub hash: Zobrist,
    pub depth: u8,
    pub score: Score,
    pub bound: Bound,
    pub mv: Move,
}

impl CacheEntry {
    /// Creates a new `CacheEntry`.
    pub fn new(hash: Zobrist, depth: usize, score: Score, bound: Bound, mv: Move) -> Self {
        Self {
            depth: depth as u8,
            hash,
            score,
            bound,
            mv,
        }
    }

    /// Adjusts the mating score of the `CacheEntry` by the given adjustment.
    ///
    /// This is used to ensure that the mating score is always the same distance from the root.
    pub fn adjust_mating_score(&mut self, adjustment: i32) {
        if self.score.is_mating() {
            self.score.0 += adjustment;
        } else if self.score.is_getting_mated() {
            self.score.0 -= adjustment;
        }
    }
}
