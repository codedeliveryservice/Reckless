use crate::types::{Bitboard, Color, Move, Piece, PieceType, Square};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 13];

#[inline(always)]
fn apply_bonus<const MAX: i32>(entry: &mut i16, bonus: i32) {
    let bonus = bonus.clamp(-MAX, MAX);
    *entry += (bonus - bonus.abs() * (*entry) as i32 / MAX) as i16;
}

#[inline(always)]
fn clamp_i16(x: i32, max_abs: i32) -> i16 {
    x.max(-max_abs).min(max_abs) as i16
}

struct QuietHistoryEntry {
    factorizer: i16,
    buckets: [[i16; 2]; 2],
}

impl QuietHistoryEntry {
    const MAX_FACTORIZER: i32 = 1940;
    const MAX_BUCKET: i32 = 6029;

    #[inline(always)]
    pub fn bucket(&self, threats: Bitboard, mv: Move) -> i16 {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;
        self.buckets[from_threated][to_threated]
    }

    #[inline(always)]
    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        let bonus = bonus.clamp(-Self::MAX_FACTORIZER, Self::MAX_FACTORIZER);
        apply_bonus::<{ Self::MAX_FACTORIZER }>(entry, bonus);
    }

    #[inline(always)]
    fn recenter_buckets(&mut self) {
        let a = self.buckets[0][0] as i32;
        let b = self.buckets[0][1] as i32;
        let c = self.buckets[1][0] as i32;
        let d = self.buckets[1][1] as i32;

        let sum = a + b + c + d;
        let mean = sum >> 2; // integer mean

        // zero-mean buckets
        self.buckets[0][0] = clamp_i16(a - mean, Self::MAX_BUCKET);
        self.buckets[0][1] = clamp_i16(b - mean, Self::MAX_BUCKET);
        self.buckets[1][0] = clamp_i16(c - mean, Self::MAX_BUCKET);
        self.buckets[1][1] = clamp_i16(d - mean, Self::MAX_BUCKET);

        // push baseline into factorizer
        let f = self.factorizer as i32 + mean;
        self.factorizer = clamp_i16(f, Self::MAX_FACTORIZER);
    }

    #[inline(always)]
    pub fn update_bucket(&mut self, threats: Bitboard, mv: Move, bonus: i32) {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        let entry = &mut self.buckets[from_threated][to_threated];
        apply_bonus::<{ Self::MAX_BUCKET }>(entry, bonus);

        // orthogonalize: keep buckets as deviations only
        self.recenter_buckets();
    }
}

pub struct QuietHistory {
    entries: Box<[FromToHistory<QuietHistoryEntry>; 2]>,
}

impl QuietHistory {
    #[inline(always)]
    pub fn get(&self, threats: Bitboard, stm: Color, mv: Move) -> i32 {
        let entry = &self.entries[stm][mv.from()][mv.to()];
        (entry.factorizer + entry.bucket(threats, mv)) as i32
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn bucket(&self, threats: Bitboard, sq: Square, captured: PieceType) -> i16 {
        let threated = threats.contains(sq) as usize;
        self.buckets[captured][threated]
    }

    #[inline(always)]
    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        apply_bonus::<{ Self::MAX_FACTORIZER }>(entry, bonus);
    }

    #[inline(always)]
    fn recenter_row(&mut self, captured: usize) {
        let x0 = self.buckets[captured][0] as i32;
        let x1 = self.buckets[captured][1] as i32;
        let sum = x0 + x1;
        let mean = sum >> 1;

        self.buckets[captured][0] = clamp_i16(x0 - mean, Self::MAX_BUCKET);
        self.buckets[captured][1] = clamp_i16(x1 - mean, Self::MAX_BUCKET);

        let f = self.factorizer as i32 + mean;
        self.factorizer = clamp_i16(f, Self::MAX_FACTORIZER);
    }

    #[inline(always)]
    pub fn update_bucket(&mut self, threats: Bitboard, sq: Square, captured: PieceType, bonus: i32) {
        let threated = threats.contains(sq) as usize;
        let entry = &mut self.buckets[captured][threated];
        apply_bonus::<{ Self::MAX_BUCKET }>(entry, bonus);

        // orthogonalize only this captured row (cheap)
        self.recenter_row(captured as usize);
    }
}

pub struct NoisyHistory {
    // [piece][to][captured_piece_type][to_threated]
    entries: Box<PieceToHistory<NoisyHistoryEntry>>,
}

impl NoisyHistory {
    #[inline(always)]
    pub fn get(&self, threats: Bitboard, piece: Piece, sq: Square, captured: PieceType) -> i32 {
        let entry = &self.entries[piece][sq];
        (entry.factorizer + entry.bucket(threats, sq, captured)) as i32
    }

    #[inline(always)]
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
    const MAX_HISTORY: i32 = 14734;

    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    #[inline(always)]
    pub fn get(&self, stm: Color, key: u64) -> i32 {
        (self.entries[stm][key as usize & Self::MASK] as i32) / 82
    }

    #[inline(always)]
    pub fn update(&mut self, stm: Color, key: u64, bonus: i32) {
        let entry = &mut self.entries[stm][key as usize & Self::MASK];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
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
    const MAX_HISTORY: i32 = 16222;

    #[inline(always)]
    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    #[inline(always)]
    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq] as i32) / 100
    }

    #[inline(always)]
    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square, bonus: i32) {
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][sq];
        apply_bonus::<{ Self::MAX_HISTORY }>(entry, bonus);
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
    const MAX_HISTORY: i32 = 15324;

    #[inline(always)]
    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    #[inline(always)]
    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq]) as i32
    }

    #[inline(always)]
    pub fn update(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square, bonus: i32) {
        let entry = &mut unsafe { &mut *subtable_ptr }[piece][sq];
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
