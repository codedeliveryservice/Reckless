use std::cell::UnsafeCell;

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
pub struct InternalEntry {
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

impl InternalEntry {
    pub const fn relative_age(&self, tt_age: u8) -> i32 {
        ((AGE_CYCLE + tt_age - self.flags.age()) & AGE_MASK) as i32
    }
}

#[derive(Clone)]
#[repr(align(32))]
struct Cluster {
    entries: [InternalEntry; ENTRIES_PER_CLUSTER],
}

/// The transposition table is used to cache previously performed search results.
pub struct TranspositionTable {
    ptr: UnsafeCell<*mut Cluster>,
    len: UnsafeCell<usize>,
    age: UnsafeCell<u8>,
}

unsafe impl Sync for TranspositionTable {}

impl TranspositionTable {
    /// Clears the transposition table. This will remove all entries but keep the allocated memory.
    pub fn clear(&self, threads: usize) {
        unsafe {
            parallel_clear(threads, self.ptr(), self.len());
            *self.age.get() = 0;
        }
    }

    /// Resizes the transposition table to the specified size in megabytes. This will clear all entries.
    pub fn resize(&self, threads: usize, megabytes: usize) {
        unsafe { deallocate(self.ptr(), self.len()) };

        let (new_ptr, new_len) = unsafe { allocate(threads, megabytes) };

        unsafe {
            *self.ptr.get() = new_ptr;
            *self.len.get() = new_len;
            *self.age.get() = 0;
        }
    }

    /// Returns the approximate load factor of the transposition table in permille (on a scale of `0` to `1000`).
    pub fn hashfull(&self) -> usize {
        let age = self.age();
        let clusters = unsafe { std::slice::from_raw_parts(self.ptr(), self.len()) };

        let mut count = 0;
        for cluster in clusters.iter().take(1000) {
            for entry in &cluster.entries {
                count += (entry.flags.bound() != Bound::None && entry.flags.age() == age) as usize;
            }
        }

        count / ENTRIES_PER_CLUSTER
    }

    pub fn increment_age(&self) {
        unsafe {
            *self.age.get() = (self.age() + 1) & AGE_MASK;
        }
    }

    pub fn read(&self, hash: u64, halfmove_clock: u8, ply: usize) -> (Option<Entry>, *const InternalEntry) {
        let cluster = {
            let index = index(hash, self.len());
            unsafe { &*self.ptr().add(index) }
        };

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

                return (Some(hit), std::ptr::from_ref(entry));
            }
        }

        let tt_age = self.age();

        let mut replacement_slot = cluster.entries.as_ptr();
        let mut lowest_quality = i32::MAX;

        for candidate in &cluster.entries {
            let quality = candidate.depth as i32 - 4 * candidate.relative_age(tt_age);

            if quality < lowest_quality {
                replacement_slot = std::ptr::from_ref(candidate);
                lowest_quality = quality;
            }
        }

        (None, replacement_slot)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &self, ptr: *const InternalEntry, hash: u64, depth: i32, eval: i32, mut score: i32, bound: Bound, mv: Move,
        ply: usize, pv: bool,
    ) {
        // Used for checking if an entry exists
        debug_assert!(depth != TtDepth::NONE);

        let entry = unsafe { &mut *ptr.cast_mut() };
        let key = verification_key(hash);
        let tt_age = self.age();

        if !(entry.key == key && mv.is_null()) {
            entry.mv = mv;
        }

        if !(key != entry.key
            || bound == Bound::Exact
            || depth + 4 + 2 * pv as i32 > entry.depth as i32
            || entry.flags.age() != tt_age)
        {
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

    pub fn refresh_age(&self, ptr: *const InternalEntry) {
        let entry = unsafe { &mut *ptr.cast_mut() };
        entry.flags = Flags::new(entry.flags.bound(), entry.flags.pv(), self.age());
    }

    pub fn prefetch(&self, hash: u64) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};

            let index = index(hash, self.len());
            let ptr = self.ptr().add(index).cast();
            _mm_prefetch::<_MM_HINT_T0>(ptr);
        }

        // No prefetching for non-x86_64 architectures
        #[cfg(not(target_arch = "x86_64"))]
        let _ = hash;
    }

    fn age(&self) -> u8 {
        unsafe { *self.age.get() }
    }

    fn ptr(&self) -> *mut Cluster {
        unsafe { *self.ptr.get() }
    }

    fn len(&self) -> usize {
        unsafe { *self.len.get() }
    }
}

const fn index(hash: u64, len: usize) -> usize {
    // Fast hash table index calculation
    // For details, see: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction
    (((hash as u128) * (len as u128)) >> 64) as usize
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
        let (ptr, len) = unsafe { allocate(1, DEFAULT_TT_SIZE) };
        Self {
            ptr: UnsafeCell::new(ptr),
            len: UnsafeCell::new(len),
            age: UnsafeCell::new(0),
        }
    }
}

impl Drop for TranspositionTable {
    fn drop(&mut self) {
        unsafe { deallocate(self.ptr(), self.len()) };
    }
}

unsafe fn allocate(threads: usize, size_mb: usize) -> (*mut Cluster, usize) {
    #[cfg(target_os = "linux")]
    use libc::{madvise, mmap, MADV_HUGEPAGE, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE};

    let size = size_mb * MEGABYTE;
    let len = size / CLUSTER_SIZE;

    #[cfg(target_os = "linux")]
    let ptr = {
        let ptr = mmap(std::ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        madvise(ptr, size, MADV_HUGEPAGE);
        ptr.cast()
    };

    #[cfg(not(target_os = "linux"))]
    let ptr = {
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<Cluster>()).unwrap();
        std::alloc::alloc_zeroed(layout).cast()
    };

    unsafe { parallel_clear(threads, ptr, len) };
    (ptr, len)
}

unsafe fn deallocate(ptr: *mut Cluster, len: usize) {
    let size = len * CLUSTER_SIZE;

    #[cfg(target_os = "linux")]
    let _ = libc::munmap(ptr.cast(), size);

    #[cfg(not(target_os = "linux"))]
    {
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<Cluster>()).unwrap();
        std::alloc::dealloc(ptr.cast(), layout);
    }
}

unsafe fn parallel_clear<T: std::marker::Send>(threads: usize, ptr: *mut T, len: usize) {
    std::thread::scope(|scope| {
        let slice = std::slice::from_raw_parts_mut(ptr, len);

        let chunk_size = len.div_ceil(threads);
        for chunk in slice.chunks_mut(chunk_size) {
            scope.spawn(|| chunk.as_mut_ptr().write_bytes(0, chunk.len()));
        }
    });
}
