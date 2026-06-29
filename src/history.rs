use std::sync::atomic::{AtomicI16, Ordering};

use crate::types::{Bitboard, Color, Move, Piece, PieceType, Square};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 13];
type ContinuationHistoryType = [[[[PieceToHistory<i16>; 64]; 13]; 2]; 2];

struct HugeBox<T> {
    ptr: std::ptr::NonNull<T>,
}

unsafe impl<T: Send> Send for HugeBox<T> {}
unsafe impl<T: Sync> Sync for HugeBox<T> {}

impl<T> HugeBox<T> {
    fn new_zeroed() -> Self {
        #[cfg(target_os = "linux")]
        let ptr = unsafe {
            use libc::{MADV_HUGEPAGE, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE, madvise, mmap};
            let size = std::mem::size_of::<T>();
            assert!(size > 0, "HugeBox requires a non-zero-sized type");
            let p = mmap(std::ptr::null_mut(), size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
            if p == MAP_FAILED {
                std::alloc::handle_alloc_error(std::alloc::Layout::new::<T>());
            }
            madvise(p, size, MADV_HUGEPAGE);
            std::ptr::NonNull::new_unchecked(p.cast::<T>())
        };

        #[cfg(not(target_os = "linux"))]
        let ptr = unsafe {
            let layout = std::alloc::Layout::new::<T>();
            let p = std::alloc::alloc_zeroed(layout);
            if p.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            std::ptr::NonNull::new_unchecked(p.cast::<T>())
        };

        HugeBox { ptr }
    }
}

impl<T> std::ops::Deref for HugeBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> std::ops::DerefMut for HugeBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Drop for HugeBox<T> {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            let size = std::mem::size_of::<T>();
            assert!(size > 0, "HugeBox requires a non-zero-sized type");
            unsafe {
                libc::munmap(self.ptr.as_ptr().cast(), size);
            }
        }

        #[cfg(not(target_os = "linux"))]
        unsafe {
            let layout = std::alloc::Layout::new::<T>();
            std::alloc::dealloc(self.ptr.as_ptr().cast(), layout);
        }
    }
}

fn apply_bonus<const MAX: i32>(entry: &mut i16, bonus: i32) {
    let bonus = bonus.clamp(-MAX, MAX);
    *entry += (bonus - bonus.abs() * (*entry) as i32 / MAX) as i16;
}

macro_rules! history_table {
    (
        $(#[$meta:meta])*
        $name:ident {
            entries: $store:ty,
            max: $max:expr,
            $(const $cname:ident: $cty:ty = $cval:expr;)*
            index($($arg:ident: $aty:ty),* $(,)?) = $([$sub:expr])+,
        }
    ) => {
        $(#[$meta])*
        pub struct $name {
            entries: Box<$store>,
        }

        impl $name {
            const MAX_HISTORY: i32 = $max;
            $(const $cname: $cty = $cval;)*

            pub fn get(&self, $($arg: $aty),*) -> i32 {
                (self.entries$([$sub])+) as i32
            }

            pub fn update(&mut self, $($arg: $aty),*, bonus: i32) {
                apply_bonus::<{ Self::MAX_HISTORY }>(&mut self.entries$([$sub])+, bonus);
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self { entries: zeroed_box() }
            }
        }
    };
}

history_table! {
    /// [side_to_move][from_threatened][to_threatened][from][to]
    QuietHistory {
        entries: [[[FromToHistory<i16>; 2]; 2]; 2],
        max: 8192,
        index(threats: Bitboard, stm: Color, mv: Move) =
            [stm][threats.contains(mv.from()) as usize][threats.contains(mv.to()) as usize][mv.from()][mv.to()],
    }
}

history_table! {
    /// [pawn_key_bucket][piece][to]
    PawnHistory {
        entries: [PieceToHistory<i16>; Self::SIZE],
        max: 8192,
        const SIZE: usize = 512;
        const MASK: usize = Self::SIZE - 1;
        index(pawn_key: u64, piece: Piece, to: Square) =
            [pawn_key as usize & Self::MASK][piece][to],
    }
}

history_table! {
    /// [piece][to][captured_piece_type][to_threatened]
    NoisyHistory {
        entries: PieceToHistory<[[i16; 2]; 7]>,
        max: 12800,
        index(threats: Bitboard, piece: Piece, sq: Square, captured: PieceType) =
            [piece][sq][captured][threats.contains(sq) as usize],
    }
}

pub struct CorrectionHistory {
    // [bucket][side_to_move][key]
    entries: Box<[[[AtomicI16; Self::SIZE]; 2]; 16]>,
}

impl CorrectionHistory {
    const MAX_HISTORY: i32 = 14605;

    const SIZE: usize = 65536;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64, bucket: usize) -> i32 {
        self.entries[bucket][stm][key as usize & Self::MASK].load(Ordering::Relaxed) as i32
    }

    pub fn update(&self, stm: Color, key: u64, bucket: usize, bonus: i32) {
        let current = self.entries[bucket][stm][key as usize & Self::MASK].load(Ordering::Relaxed) as i32;
        let new = current + bonus - bonus.abs() * current / Self::MAX_HISTORY;
        self.entries[bucket][stm][key as usize & Self::MASK].store(new as i16, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        for bucket in self.entries.iter() {
            for entries in bucket.iter() {
                for entry in entries {
                    entry.store(0, Ordering::Relaxed);
                }
            }
        }
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationTable<const MAX: i32> {
    entries: HugeBox<ContinuationHistoryType>,
}

impl<const MAX: i32> ContinuationTable<MAX> {
    pub fn subtable_ptr(
        &mut self, in_check: bool, capture: bool, piece: Piece, to: Square,
    ) -> *mut PieceToHistory<i16> {
        &raw mut self.entries[in_check as usize][capture as usize][piece][to]
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square) -> i32 {
        unsafe { (&*subtable_ptr)[piece][to] as i32 }
    }

    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square, bonus: i32) {
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][to];
        apply_bonus::<MAX>(entry, bonus);
    }
}

impl<const MAX: i32> Default for ContinuationTable<MAX> {
    fn default() -> Self {
        Self { entries: HugeBox::new_zeroed() }
    }
}

pub type ContinuationCorrectionHistory = ContinuationTable<16418>;
pub type ContinuationHistory = ContinuationTable<15320>;

fn zeroed_box<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::<T>::from_raw(ptr.cast())
    }
}
