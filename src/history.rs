use crate::{
    parameters::*,
    types::{Bitboard, Color, Move, Piece, PieceType, Square},
};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 13];

fn apply_bonus(entry: &mut i16, bonus: i32, max: i32) {
    let bonus = bonus.clamp(-max, max);
    *entry += (bonus - bonus.abs() * (*entry) as i32 / max) as i16;
}

struct QuietHistoryEntry {
    factorizer: i16,
    buckets: [[i16; 2]; 2],
}

impl QuietHistoryEntry {
    pub fn bucket(&self, threats: Bitboard, mv: Move) -> i16 {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        self.buckets[from_threated][to_threated]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let max = max1();
        let entry = &mut self.factorizer;
        let bonus = bonus.clamp(-max, max);
        apply_bonus(entry, bonus, max);
    }

    pub fn update_bucket(&mut self, threats: Bitboard, mv: Move, bonus: i32) {
        let max = max2();
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        let entry = &mut self.buckets[from_threated][to_threated];
        apply_bonus(entry, bonus, max);
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
    pub fn bucket(&self, threats: Bitboard, sq: Square, captured: PieceType) -> i16 {
        let threated = threats.contains(sq) as usize;
        self.buckets[captured][threated]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let max = max3();
        let entry = &mut self.factorizer;
        apply_bonus(entry, bonus, max);
    }

    pub fn update_bucket(&mut self, threats: Bitboard, sq: Square, captured: PieceType, bonus: i32) {
        let max = max4();
        let threated = threats.contains(sq) as usize;
        let entry = &mut self.buckets[captured][threated];
        apply_bonus(entry, bonus, max);
    }
}

pub struct NoisyHistory {
    // [piece][to][captured_piece_type][to_threated]
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
    entries: Box<[[i16; Self::SIZE]; 2]>,
}

impl CorrectionHistory {
    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        (self.entries[stm][key as usize & Self::MASK] as i32) / corr4()
    }

    pub fn update(&mut self, stm: Color, key: u64, bonus: i32) {
        let entry = &mut self.entries[stm][key as usize & Self::MASK];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / max5()) as i16;
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationCorrectionHistory {
    // [piece][to][continuation_piece][continuation_to]
    entries: Box<[[PieceToHistory<i16>; 64]; 13]>,
}

impl ContinuationCorrectionHistory {
    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq] as i32) / corr5()
    }

    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square, bonus: i32) {
        let max = max6();
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][sq];
        apply_bonus(entry, bonus, max);
    }
}

impl Default for ContinuationCorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationHistory {
    // [piece][to][continuation_piece][continuation_to]
    entries: Box<[[PieceToHistory<i16>; 64]; 13]>,
}

impl ContinuationHistory {
    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq]) as i32
    }

    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square, bonus: i32) {
        let max = max7();
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][sq];
        apply_bonus(entry, bonus, max);
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
