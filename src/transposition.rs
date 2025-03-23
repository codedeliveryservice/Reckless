use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicI16, AtomicU16, AtomicU8, Ordering},
};

use crate::types::{is_decisive, Move};

pub const DEFAULT_TT_SIZE: usize = 16;

const MEGABYTE: usize = 1024 * 1024;
const INTERNAL_ENTRY_SIZE: usize = std::mem::size_of::<Block>();

const _: () = assert!(INTERNAL_ENTRY_SIZE == 10, "InternalEntry size is not 10 bytes");

#[derive(Copy, Clone)]
pub struct Entry {
    pub mv: Move,
    pub score: i32,
    pub eval: i32,
    pub depth: i32,
    pub bound: Bound,
    pub pv: bool,
}

pub struct Flags {
    data: u8,
}

impl Flags {
    pub const fn new(bound: Bound, pv: bool) -> Self {
        Self { data: bound as u8 | ((pv as u8) << 2) }
    }

    pub const fn bound(&self) -> Bound {
        unsafe { std::mem::transmute(self.data & 0b11) }
    }

    pub const fn pv(&self) -> bool {
        (self.data & 0b100) != 0
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
struct InternalEntry {
    key: u16,     // 2 bytes
    mv: Move,     // 2 bytes
    score: i16,   // 2 bytes
    eval: i16,    // 2 bytes
    depth: u8,    // 1 byte
    flags: Flags, // 1 byte
}

#[derive(Default)]
struct Block {
    key: AtomicU16,
    mv: AtomicU16,
    score: AtomicI16,
    eval: AtomicI16,
    depth: AtomicU8,
    flags: AtomicU8,
}

impl Block {
    fn load(&self) -> InternalEntry {
        InternalEntry {
            key: self.key.load(Ordering::Relaxed),
            mv: Move(self.mv.load(Ordering::Relaxed)),
            score: self.score.load(Ordering::Relaxed) as i16,
            eval: self.eval.load(Ordering::Relaxed) as i16,
            depth: self.depth.load(Ordering::Relaxed),
            flags: Flags { data: self.flags.load(Ordering::Relaxed) },
        }
    }

    fn write(&self, entry: InternalEntry) {
        self.key.store(entry.key, Ordering::Relaxed);
        self.mv.store(entry.mv.0, Ordering::Relaxed);
        self.eval.store(entry.eval, Ordering::Relaxed);
        self.score.store(entry.score, Ordering::Relaxed);
        self.depth.store(entry.depth, Ordering::Relaxed);
        self.flags.store(entry.flags.data, Ordering::Relaxed);
    }
}

impl Clone for Block {
    fn clone(&self) -> Self {
        Self {
            key: AtomicU16::new(self.key.load(Ordering::Relaxed)),
            mv: AtomicU16::new(self.mv.load(Ordering::Relaxed)),
            score: AtomicI16::new(self.score.load(Ordering::Relaxed)),
            eval: AtomicI16::new(self.eval.load(Ordering::Relaxed)),
            depth: AtomicU8::new(self.depth.load(Ordering::Relaxed)),
            flags: AtomicU8::new(self.flags.load(Ordering::Relaxed)),
        }
    }
}

/// The transposition table is used to cache previously performed search results.
pub struct TranspositionTable {
    vector: UnsafeCell<Vec<Block>>,
}

unsafe impl Sync for TranspositionTable {}

impl TranspositionTable {
    /// Clears the transposition table. This will remove all entries but keep the allocated memory.
    pub fn clear(&self, threads: usize) {
        unsafe { self.parallel_clear(threads, self.len()) }
    }

    /// Resizes the transposition table to the specified size in megabytes. This will clear all entries.
    pub fn resize(&self, threads: usize, megabytes: usize) {
        let len = megabytes * MEGABYTE / INTERNAL_ENTRY_SIZE;

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
        vector.iter().take(1000).filter(|slot| slot.load().flags.bound() != Bound::None).count()
    }

    pub fn read(&self, hash: u64, ply: usize) -> Option<Entry> {
        let entry = self.entry(hash).load();

        if entry.flags.bound() == Bound::None || entry.key != verification_key(hash) {
            return None;
        }

        let mut hit = Entry {
            depth: entry.depth as i32,
            score: entry.score as i32,
            eval: entry.eval as i32,
            bound: entry.flags.bound(),
            pv: entry.flags.pv(),
            mv: entry.mv,
        };

        // Adjust mate distance from "plies from the current position" to "plies from the root"
        if is_decisive(hit.score) {
            hit.score -= hit.score.signum() * ply as i32;
        }

        Some(hit)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &self, hash: u64, depth: i32, eval: i32, mut score: i32, bound: Bound, mv: Move, ply: usize, pv: bool,
    ) {
        // Adjust mate distance from "plies from the root" to "plies from the current position"
        if is_decisive(score) {
            score += score.signum() * ply as i32;
        }

        let key = verification_key(hash);
        let entry = self.entry(hash);
        let stored = entry.load();

        let mv = if stored.key == key && mv.is_null() { stored.mv } else { mv };

        entry.write(InternalEntry {
            key,
            depth: depth as u8,
            score: score as i16,
            eval: eval as i16,
            flags: Flags::new(bound, pv),
            mv,
        });
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

    fn len(&self) -> usize {
        unsafe { (*self.vector.get()).len() }
    }

    fn entry(&self, hash: u64) -> &Block {
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
            vector: UnsafeCell::new(vec![Block::default(); DEFAULT_TT_SIZE * MEGABYTE / INTERNAL_ENTRY_SIZE]),
        }
    }
}
