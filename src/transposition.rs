use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};

use crate::types::{is_decisive, Move};

pub const DEFAULT_TT_SIZE: usize = 16;

const MEGABYTE: usize = 1024 * 1024;
const CLUSTER_SIZE: usize = std::mem::size_of::<InternalEntry>() * CLUSTERS;

const CLUSTERS: usize = 4;
const MAX_AGE: u8 = 32;

#[derive(Copy, Clone)]
pub struct Entry {
    pub mv: Move,
    pub score: i32,
    pub depth: i32,
    pub bound: Bound,
    pub pv: bool,
}

pub struct Flags {
    data: u8,
}

impl Flags {
    pub const fn new(bound: Bound, pv: bool, age: u8) -> Self {
        Self { data: (bound as u8) | ((pv as u8) << 2) | (age << 3) }
    }

    pub const fn bound(&self) -> Bound {
        unsafe { std::mem::transmute(self.data & 0b11) }
    }

    pub const fn pv(&self) -> bool {
        (self.data & 0b100) != 0
    }

    pub const fn age(&self) -> u8 {
        self.data >> 3
    }
}

/// Type of the score returned by the search.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Bound {
    None,
    Exact,
    Lower,
    Upper,
}

/// Internal representation of a transposition table entry (8 bytes).
#[repr(C, align(8))]
struct InternalEntry {
    key: u16,     // 2 bytes
    mv: Move,     // 2 bytes
    score: i16,   // 2 bytes
    depth: u8,    // 1 byte
    flags: Flags, // 1 byte
}

impl InternalEntry {
    pub fn is_empty(&self) -> bool {
        self.flags.bound() == Bound::None
    }

    pub fn quality(&self, age: u8) -> i32 {
        self.depth as i32 - 4 * (age - self.flags.age()) as i32
    }
}

#[derive(Default)]
#[repr(C, align(32))]
struct Cluster {
    inner: [AtomicU64; CLUSTERS],
}

impl Cluster {
    fn load(&self, index: usize) -> InternalEntry {
        let entry = self.inner[index].load(Ordering::Relaxed);
        unsafe { std::mem::transmute::<u64, InternalEntry>(entry) }
    }

    fn write(&self, entry: InternalEntry, index: usize) {
        let entry = unsafe { std::mem::transmute::<InternalEntry, u64>(entry) };
        self.inner[index].store(entry, Ordering::Relaxed);
    }
}

impl Clone for Cluster {
    fn clone(&self) -> Self {
        Self {
            inner: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
        }
    }
}

/// The transposition table is used to cache previously performed search results.
pub struct TranspositionTable {
    vector: UnsafeCell<Vec<Cluster>>,
    age: AtomicU8,
}

unsafe impl Sync for TranspositionTable {}

impl TranspositionTable {
    /// Clears the transposition table. This will remove all entries but keep the allocated memory.
    pub fn clear(&self, threads: usize) {
        unsafe { self.parallel_clear(threads, self.len()) }
    }

    /// Resizes the transposition table to the specified size in megabytes. This will clear all entries.
    pub fn resize(&self, threads: usize, megabytes: usize) {
        let len = megabytes * MEGABYTE / CLUSTER_SIZE;

        let mut vector = Vec::new();
        vector.reserve_exact(len);

        unsafe {
            let vec = &mut *self.vector.get();

            drop(std::ptr::replace(vec, vector));

            self.parallel_clear(threads, len);
            vec.set_len(len);
        }
    }

    /// Returns the approximate load factor of the transposition table in permille (on a scale of `0` to `1000`).
    pub fn hashfull(&self) -> usize {
        let vector = unsafe { &*self.vector.get() };

        let mut count = 0;
        for cluster in vector.iter() {
            for i in 0..CLUSTERS {
                count += !cluster.load(i).is_empty() as usize;
            }
        }
        count * 1000 / vector.len() / CLUSTERS
    }

    pub fn increment_age(&self) {
        self.age.store((self.age() + 1) & (MAX_AGE - 1), Ordering::Relaxed);
    }

    pub fn read(&self, hash: u64, ply: usize) -> Option<Entry> {
        let cluster = self.cluster(hash);

        for index in 0..CLUSTERS {
            let entry = cluster.load(index);
            if entry.is_empty() || entry.key != verification_key(hash) {
                continue;
            }

            let mut hit = Entry {
                depth: entry.depth as i32,
                score: entry.score as i32,
                bound: entry.flags.bound(),
                pv: entry.flags.pv(),
                mv: entry.mv,
            };

            // Adjust mate distance from "plies from the current position" to "plies from the root"
            if is_decisive(hit.score) {
                hit.score -= hit.score.signum() * ply as i32;
            }

            return Some(hit);
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write(&self, hash: u64, depth: i32, mut score: i32, bound: Bound, mv: Move, ply: usize, pv: bool) {
        // Adjust mate distance from "plies from the root" to "plies from the current position"
        if is_decisive(score) {
            score += score.signum() * ply as i32;
        }

        let age = self.age();
        let key = verification_key(hash);
        let cluster = self.cluster(hash);

        let mut index = 0;
        let mut minimum = i32::MAX;

        for i in 0..CLUSTERS {
            let current = cluster.load(i);
            let quality = current.quality(age);

            if current.is_empty() || current.key == key {
                index = i;
                break;
            }

            if quality < minimum {
                index = i;
                minimum = quality;
            }
        }

        let mut entry = cluster.load(index);

        if !(entry.key != key
            || depth + 4 + 2 * pv as i32 > entry.depth as i32
            || bound == Bound::Exact
            || entry.flags.age() != self.age())
        {
            return;
        }

        if !(entry.key == key && mv == Move::NULL) {
            entry.mv = mv;
        }

        entry.key = key;
        entry.depth = depth as u8;
        entry.score = score as i16;
        entry.flags = Flags::new(bound, pv, self.age());

        cluster.write(entry, index);
    }

    pub fn prefetch(&self, hash: u64) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

            let index = self.index(hash);
            let ptr = (*self.vector.get()).as_ptr().add(index).cast();
            _mm_prefetch::<_MM_HINT_T0>(ptr);
        }

        // No prefetching for non-x86_64 architectures
        #[cfg(not(target_arch = "x86_64"))]
        let _ = hash;
    }

    fn age(&self) -> u8 {
        self.age.load(Ordering::Relaxed)
    }

    fn len(&self) -> usize {
        unsafe { (*self.vector.get()).len() }
    }

    fn cluster(&self, hash: u64) -> &Cluster {
        let index = self.index(hash);
        unsafe {
            let vec = &*self.vector.get();
            vec.get_unchecked(index)
        }
    }

    fn index(&self, hash: u64) -> usize {
        // Fast hash table index calculation
        // For details, see: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
        (((hash as u128) * (self.len() as u128)) >> 64) as usize
    }

    unsafe fn parallel_clear(&self, threads: usize, len: usize) {
        std::thread::scope(|scope| {
            let vec = &mut *self.vector.get();
            let ptr = vec.as_mut_ptr();
            let slice = std::slice::from_raw_parts_mut(ptr, len);

            let chunk_size = len.div_ceil(threads);
            for chunk in slice.chunks_mut(chunk_size) {
                scope.spawn(|| chunk.as_mut_ptr().write_bytes(0, chunk.len()));
            }
        });
    }
}

/// Returns the verification key of the hash (bottom 16 bits).
const fn verification_key(hash: u64) -> u16 {
    hash as u16
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            vector: UnsafeCell::new(vec![Cluster::default(); DEFAULT_TT_SIZE * MEGABYTE / CLUSTER_SIZE]),
            age: AtomicU8::new(0),
        }
    }
}
