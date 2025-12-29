use std::sync::atomic::{AtomicI16, Ordering};

use crate::types::{Bitboard, Color, Move, Piece, PieceType, Square};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 13];
type ContinuationHistoryType = [[[[PieceToHistory<i16>; 64]; 13]; 2]; 2];

fn apply_bonus<const MAX: i32>(entry: &mut i16, bonus: i32) {
    let bonus = bonus.clamp(-MAX, MAX);
    *entry += (bonus - bonus.abs() * (*entry) as i32 / MAX) as i16;
}

struct QuietHistoryEntry {
    factorizer: i16,
    buckets: [[i16; 2]; 2],
}

impl QuietHistoryEntry {
    const MAX_FACTORIZER: i32 = 1940;
    const MAX_BUCKET: i32 = 6029;

    pub fn bucket(&self, threats: Bitboard, mv: Move) -> i16 {
        let from_threatened = threats.contains(mv.from()) as usize;
        let to_threatened = threats.contains(mv.to()) as usize;

        self.buckets[from_threatened][to_threatened]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        apply_bonus::<{ Self::MAX_FACTORIZER }>(entry, bonus);
    }

    pub fn update_bucket(&mut self, threats: Bitboard, mv: Move, bonus: i32) {
        let from_threatened = threats.contains(mv.from()) as usize;
        let to_threatened = threats.contains(mv.to()) as usize;

        let entry = &mut self.buckets[from_threatened][to_threatened];
        apply_bonus::<{ Self::MAX_BUCKET }>(entry, bonus);
    }
}

pub struct QuietHistory {
    entries: Box<[FromToHistory<QuietHistoryEntry>; 2]>,
}

impl QuietHistory {
    pub fn get(&self, threats: Bitboard, stm: Color, mv: Move) -> i32 {
        let entry = &self.entries[stm][mv.from()][mv.to()];
        (entry.factorizer + entry.bucket(threats, mv)) as i32
    }

    pub fn update(&mut self, threats: Bitboard, stm: Color, mv: Move, bonus: i32) {
        let entry = &mut self.entries[stm][mv.from()][mv.to()];

        entry.update_factorizer(bonus);
        entry.update_bucket(threats, mv, bonus);
    }
}

impl Default for QuietHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

struct NoisyHistoryEntry {
    factorizer: i16,
    buckets: [[i16; 2]; 7],
}

impl NoisyHistoryEntry {
    const MAX_FACTORIZER: i32 = 4449;
    const MAX_BUCKET: i32 = 8148;

    pub fn bucket(&self, threats: Bitboard, sq: Square, captured: PieceType) -> i16 {
        let threatened = threats.contains(sq) as usize;
        self.buckets[captured][threatened]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        apply_bonus::<{ Self::MAX_FACTORIZER }>(entry, bonus);
    }

    pub fn update_bucket(&mut self, threats: Bitboard, sq: Square, captured: PieceType, bonus: i32) {
        let threatened = threats.contains(sq) as usize;
        let entry = &mut self.buckets[captured][threatened];
        apply_bonus::<{ Self::MAX_BUCKET }>(entry, bonus);
    }
}

pub struct NoisyHistory {
    // [piece][to][captured_piece_type][to_threatened]
    entries: Box<PieceToHistory<NoisyHistoryEntry>>,
}

impl NoisyHistory {
    pub fn get(&self, threats: Bitboard, piece: Piece, sq: Square, captured: PieceType) -> i32 {
        let entry = &self.entries[piece][sq];
        (entry.factorizer + entry.bucket(threats, sq, captured)) as i32
    }

    pub fn update(&mut self, threats: Bitboard, piece: Piece, sq: Square, captured: PieceType, bonus: i32) {
        let entry = &mut self.entries[piece][sq];

        entry.update_factorizer(bonus);
        entry.update_bucket(threats, sq, captured, bonus);
    }
}

impl Default for NoisyHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct CorrectionHistory {
    // [side_to_move][key]
    entries: Box<[Box<[AtomicI16]>; 2]>,
    size: usize,
}

unsafe impl Sync for CorrectionHistory {}

impl CorrectionHistory {
    const MAX_HISTORY: i32 = 14734;
    const BASE_SIZE: usize = 16384;

    pub fn new(threads: usize) -> Self {
        let size = threads.next_power_of_two() * Self::BASE_SIZE;

        let mut entries = Vec::with_capacity(2);
        for _ in 0..2 {
            let arr: Vec<AtomicI16> = (0..size).map(|_| AtomicI16::new(0)).collect();
            entries.push(arr.into_boxed_slice().try_into().unwrap());
        }

        Self {
            entries: entries.into_boxed_slice().try_into().unwrap(),
            size,
        }
    }

    fn mask(&self) -> usize {
        self.size - 1
    }

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        let mask = self.mask();
        self.entries[stm][key as usize & mask].load(Ordering::Relaxed) as i32
    }

    pub fn update(&self, stm: Color, key: u64, bonus: i32) {
        let mask = self.mask();
        let current = self.entries[stm][key as usize & mask].load(Ordering::Relaxed) as i32;
        let new = current + bonus - bonus.abs() * current / Self::MAX_HISTORY;
        self.entries[stm][key as usize & mask].store(new as i16, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        for entries in self.entries.iter() {
            for entry in entries.iter() {
                entry.store(0, Ordering::Relaxed);
            }
        }
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self {
            entries: Box::new([
                Box::new([(); Self::BASE_SIZE].map(|_| AtomicI16::new(0))),
                Box::new([(); Self::BASE_SIZE].map(|_| AtomicI16::new(0))),
            ]),
            size: Self::BASE_SIZE,
        }
    }
}

pub struct ContinuationCorrectionHistory {
    // [in_check][capture][piece][to][piece][to]
    entries: Box<ContinuationHistoryType>,
}

impl ContinuationCorrectionHistory {
    const MAX_HISTORY: i32 = 16222;

    pub fn subtable_ptr(
        &mut self, in_check: bool, capture: bool, piece: Piece, to: Square,
    ) -> *mut PieceToHistory<i16> {
        self.entries[in_check as usize][capture as usize][piece][to].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square) -> i32 {
        unsafe { (&*subtable_ptr)[piece][to] as i32 }
    }

    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square, bonus: i32) {
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][to];
        apply_bonus::<{ Self::MAX_HISTORY }>(entry, bonus);
    }
}

impl Default for ContinuationCorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationHistory {
    // [in_check][capture][piece][to][piece][to]
    entries: Box<ContinuationHistoryType>,
}

impl ContinuationHistory {
    const MAX_HISTORY: i32 = 15324;

    pub fn subtable_ptr(
        &mut self, in_check: bool, capture: bool, piece: Piece, to: Square,
    ) -> *mut PieceToHistory<i16> {
        self.entries[in_check as usize][capture as usize][piece][to].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][to]) as i32
    }

    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, to: Square, bonus: i32) {
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][to];
        apply_bonus::<{ Self::MAX_HISTORY }>(entry, bonus);
    }
}

impl Default for ContinuationHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

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
