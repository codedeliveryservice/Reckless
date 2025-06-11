use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicU8, Ordering},
};

use crate::types::{is_decisive, is_loss, is_valid, is_win, Move, Score};

pub const DEFAULT_TT_SIZE: usize = 16;

const MEGABYTE: usize = 1024 * 1024;
const CLUSTER_SIZE: usize = std::mem::size_of::<Cluster>();

const ENTRIES_PER_CLUSTER: usize = 3;

const AGE_CYCLE: u8 = 1 << 5;
const AGE_MASK: u8 = AGE_CYCLE - 1;

const _: () = assert!(std::mem::size_of::<Cluster>() == 32);
const _: () = assert!(std::mem::size_of::<InternalEntry>() == 10);

#[derive(Copy, Clone)]
pub struct Entry {
    pub mv: Move,
    pub score: i32,
    pub eval: i32,
    pub depth: i32,
    pub bound: Bound,
    pub pv: bool,
}

#[derive(Copy, Clone)]
pub struct Flags {
    data: u8,
}

impl Flags {
    pub const fn new(bound: Bound, pv: bool, age: u8) -> Self {
        Self { data: (bound as u8) | ((pv as u8) << 2) | (age << 3) }
    }

    pub const fn bound(self) -> Bound {
        unsafe { std::mem::transmute(self.data & 0b11) }
    }

    pub const fn pv(self) -> bool {
        (self.data & 0b100) != 0
    }

    pub const fn age(self) -> u8 {
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

/// Internal representation of a transposition table entry (10 bytes).
#[derive(Clone)]
#[repr(C)]
struct InternalEntry {
    key: u16,     // 2 bytes
    mv: Move,     // 2 bytes
    score: i16,   // 2 bytes
    eval: i16,    // 2 bytes
    depth: i8,    // 1 byte
    flags: Flags, // 1 byte
}

pub enum TtDepth {}

impl TtDepth {
    pub const NONE: i32 = 0;
    pub const SOME: i32 = -1;
}

impl Default for InternalEntry {
    fn default() -> Self {
        Self {
            mv: Move::NULL,
            key: 0,
            eval: Score::NONE as i16,
            score: Score::NONE as i16,
            depth: TtDepth::NONE as i8,
            flags: Flags::new(Bound::None, false, 0),
        }
    }
}

impl InternalEntry {
    pub const fn relative_age(&self, tt_age: u8) -> i32 {
        ((AGE_CYCLE + tt_age - self.flags.age()) & AGE_MASK) as i32
    }
}

#[derive(Clone, Default)]
#[repr(align(32))]
struct Cluster {
    entries: [InternalEntry; ENTRIES_PER_CLUSTER],
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
        self.age.store(0, Ordering::Relaxed);
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
        let tt_age = self.tt_age();

        let mut count = 0;
        for cluster in vector.iter().take(1000) {
            for entry in &cluster.entries {
                count += (entry.flags.bound() != Bound::None && entry.flags.age() == tt_age) as usize;
            }
        }

        count / ENTRIES_PER_CLUSTER
    }

    pub fn increment_age(&self) {
        self.age.store((self.tt_age() + 1) & AGE_MASK, Ordering::Relaxed);
    }

    pub fn read(&self, hash: u64, halfmove_clock: u8, ply: usize) -> Option<Entry> {
        let cluster = self.entry(hash);
        let key = verification_key(hash);

        for entry in &cluster.entries {
            if key == entry.key && entry.depth != TtDepth::NONE as i8 {
                let hit = Entry {
                    depth: entry.depth as i32,
                    score: score_from_tt(entry.score as i32, ply, halfmove_clock),
                    eval: entry.eval as i32,
                    bound: entry.flags.bound(),
                    pv: entry.flags.pv(),
                    mv: entry.mv,
                };

                return Some(hit);
            }
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &self, hash: u64, depth: i32, eval: i32, mut score: i32, bound: Bound, mv: Move, ply: usize, pv: bool,
    ) {
        // Used for checking if an entry exists
        debug_assert!(depth != TtDepth::NONE);

        let index = self.index(hash);
        let cluster = unsafe {
            let vector = &mut *self.vector.get();
            vector.get_unchecked_mut(index)
        };

        let key = verification_key(hash);
        let tt_age = self.tt_age();

        let mut index = 0;
        let mut minimum = i32::MAX;

        for (i, candidate) in cluster.entries.iter().enumerate() {
            if candidate.key == key || candidate.flags.bound() == Bound::None {
                index = i;
                break;
            }

            let quality = candidate.depth as i32 - 4 * candidate.relative_age(tt_age);

            if quality < minimum {
                index = i;
                minimum = quality;
            }
        }

        let entry = &mut cluster.entries[index];

        if !(entry.key == key && mv.is_null()) {
            entry.mv = mv;
        }

        if !(key != entry.key
            || bound == Bound::Exact
            || depth + 4 + 2 * pv as i32 > entry.depth as i32
            || entry.flags.age() != tt_age)
        {
            if entry.depth >= 5 && entry.flags.bound() != Bound::Exact {
                entry.depth -= 1;
            }

            return;
        }

        // Adjust mate distance from "plies from the root" to "plies from the current position"
        if is_decisive(score) && is_valid(score) {
            score += score.signum() * ply as i32;
        }

        entry.key = key;
        entry.depth = depth as i8;
        entry.score = score as i16;
        entry.eval = eval as i16;
        entry.flags = Flags::new(bound, pv, tt_age);
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

    fn tt_age(&self) -> u8 {
        self.age.load(Ordering::Relaxed)
    }

    fn len(&self) -> usize {
        unsafe { (*self.vector.get()).len() }
    }

    fn entry(&self, hash: u64) -> &Cluster {
        let index = self.index(hash);
        unsafe {
            let vector = &*self.vector.get();
            vector.get_unchecked(index)
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

/// Adjust mate distance from "plies from the root" to "plies from the current position".
const fn score_from_tt(score: i32, ply: usize, halfmove_clock: u8) -> i32 {
    if score == Score::NONE {
        return Score::NONE;
    }

    // Handle TB win or better
    if is_win(score) {
        // Downgrade a potentially false mate score
        if score >= Score::MATE_IN_MAX && Score::MATE - score > 100 - halfmove_clock as i32 {
            return Score::TB_WIN_IN_MAX - 1;
        }

        // Downgrade a potentially false TB score.
        if Score::TB_WIN - score > 100 - halfmove_clock as i32 {
            return Score::TB_WIN_IN_MAX - 1;
        }

        return score - ply as i32;
    }

    // Handle TB loss or worse
    if is_loss(score) {
        // Downgrade a potentially false mate score.
        if score <= -Score::MATE_IN_MAX && Score::MATE + score > 100 - halfmove_clock as i32 {
            return -Score::TB_WIN_IN_MAX + 1;
        }

        // Downgrade a potentially false TB score.
        if Score::TB_WIN + score > 100 - halfmove_clock as i32 {
            return -Score::TB_WIN_IN_MAX + 1;
        }

        return score + ply as i32;
    }

    score
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self {
            vector: UnsafeCell::new(vec![Cluster::default(); DEFAULT_TT_SIZE * MEGABYTE / CLUSTER_SIZE]),
            age: AtomicU8::new(0),
        }
    }
}
