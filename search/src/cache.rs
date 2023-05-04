use game::{Move, Score, Zobrist};

/// The transposition hash table is used to cache previously performed search results.
pub struct Cache {
    vector: Vec<Option<CacheEntry>>,
}

impl Cache {
    pub const DEFAULT_SIZE: usize = 16;
    pub const MAX_SIZE: usize = 512;
    pub const MIN_SIZE: usize = 1;

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

    /// Returns `Some(T)` if the entry was found; otherwise `None`.
    #[inline(always)]
    pub fn read(&self, hash: Zobrist) -> Option<CacheEntry> {
        let key = self.get_key(hash);
        match self.vector[key] {
            // Several positions can refer to one key, so check that this is it
            Some(entry) if entry.hash == hash => Some(entry),
            _ => None,
        }
    }

    /// Writes an entry to the `Cache` overwriting an existing one.
    #[inline(always)]
    pub fn write(&mut self, entry: CacheEntry) {
        let key = self.get_key(entry.hash);
        self.vector[key] = Some(entry);
    }

    #[inline(always)]
    fn get_key(&self, hash: Zobrist) -> usize {
        hash.0 as usize % self.vector.len()
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(Self::DEFAULT_SIZE)
    }
}

#[derive(Copy, Clone)]
pub enum NodeKind {
    PV,  // Principle variation node (exact score)
    Cut, // Fail-high node (beta upper bound)
    All, // Fail-low node (alpha lower bound)
}

#[derive(Copy, Clone)]
pub struct CacheEntry {
    pub hash: Zobrist,
    pub depth: u8,
    pub score: Score,
    pub kind: NodeKind,
    pub best: Move,
}

impl CacheEntry {
    /// Creates a new `CacheEntry`.
    pub fn new(hash: Zobrist, depth: usize, score: Score, kind: NodeKind, best: Move) -> Self {
        Self {
            depth: depth as u8,
            hash,
            score,
            kind,
            best,
        }
    }

    /// Returns `Some(Score)` if the `CacheEntry` is good enough compared to the `SearchParams`.
    pub fn get_score(&self, alpha: Score, beta: Score, depth: usize) -> Option<Score> {
        if self.depth < depth as u8 {
            return None;
        }

        match self.kind {
            NodeKind::PV => Some(self.score),
            NodeKind::All if self.score <= alpha => Some(alpha),
            NodeKind::Cut if self.score >= beta => Some(beta),
            _ => None,
        }
    }
}
