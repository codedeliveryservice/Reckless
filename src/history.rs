use crate::types::{Bitboard, Color, Move, Piece, PieceType, Square};

type FromToHistory<T> = [[T; 64]; 64];
type PieceToHistory<T> = [[T; 64]; 12];

struct QuietHistoryEntry {
    factorizer: i16,
    buckets: [[i16; 2]; 2],
}

impl QuietHistoryEntry {
    const MAX_FACTORIZER: i32 = 4096;
    const MAX_BUCKET: i32 = 8192;

    pub fn bucket(&self, threats: Bitboard, mv: Move) -> i16 {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        self.buckets[from_threated][to_threated]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        let bonus = bonus.clamp(-Self::MAX_FACTORIZER, Self::MAX_FACTORIZER);
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_FACTORIZER) as i16;
    }

    pub fn update_bucket(&mut self, threats: Bitboard, mv: Move, bonus: i32) {
        let from_threated = threats.contains(mv.from()) as usize;
        let to_threated = threats.contains(mv.to()) as usize;

        let entry = &mut self.buckets[from_threated][to_threated];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_BUCKET) as i16;
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
    const MAX_FACTORIZER: i32 = 4096;
    const MAX_BUCKET: i32 = 8192;

    pub fn bucket(&self, threats: Bitboard, sq: Square, captured: PieceType) -> i16 {
        let threated = threats.contains(sq) as usize;
        self.buckets[captured][threated]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_FACTORIZER) as i16;
    }

    pub fn update_bucket(&mut self, threats: Bitboard, sq: Square, captured: PieceType, bonus: i32) {
        let threated = threats.contains(sq) as usize;
        let entry = &mut self.buckets[captured][threated];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_BUCKET) as i16;
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
    const MAX_HISTORY: i32 = 16384;

    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        (self.entries[stm][key as usize & Self::MASK] / 96) as i32
    }

    pub fn update(&mut self, stm: Color, key: u64, depth: i32, diff: i32) {
        let entry = &mut self.entries[stm][key as usize & Self::MASK];
        let bonus = (diff * depth).clamp(-Self::MAX_HISTORY / 4, Self::MAX_HISTORY / 4);

        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct ContinuationHistory {
    // [piece][to][continuation_piece][continuation_to]
    entries: Box<[[PieceToHistory<i16>; 64]; 13]>,
}

impl ContinuationHistory {
    const MAX_HISTORY: i32 = 16384;

    pub fn get(&self, piece: Piece, sq: Square, cont_piece: Piece, cont_sq: Square) -> i32 {
        self.entries[piece][sq][cont_piece][cont_sq] as i32
    }

    pub fn update(&mut self, piece: Piece, sq: Square, cont_piece: Piece, cont_sq: Square, bonus: i32) {
        let entry = &mut self.entries[piece][sq][cont_piece][cont_sq];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for ContinuationHistory {
    fn default() -> Self {
        Self { entries: zeroed_box() }
    }
}

pub struct NullMoveHistory {
    entries: Box<[FromToHistory<i16>; 2]>,
}

impl NullMoveHistory {
    const MAX_HISTORY: i32 = 1024;

    pub fn get(&self, stm: Color, mv: Move) -> i32 {
        self.entries[stm][mv.from()][mv.to()] as i32
    }

    pub fn update(&mut self, stm: Color, mv: Move, bonus: i32) {
        let entry = &mut self.entries[stm][mv.from()][mv.to()];
        *entry += (bonus - bonus.abs() * (*entry) as i32 / Self::MAX_HISTORY) as i16;
    }
}

impl Default for NullMoveHistory {
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
