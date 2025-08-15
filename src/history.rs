use crate::types::{Bitboard, Color, Move, Piece, PieceType, Square};

type PieceToHistory<T> = [[T; 64]; 13];

fn apply_bonus<const MAX: i32>(entry: &mut i16, bonus: i32) {
    let bonus = bonus.clamp(-MAX, MAX);
    *entry += (bonus - bonus.abs() * (*entry) as i32 / MAX) as i16;
}

pub struct QuietHistory {
    from_bias_by_side: Box<[[i16; 64]; 2]>,
    to_bias_by_side: Box<[[i16; 64]; 2]>,
    side_bias: [i16; 2],
    weight_no_threat: [i16; 2],
    weight_from_threat: [i16; 2],
    weight_to_threat: [i16; 2],
    weight_both_threat: [i16; 2],
}

impl Default for QuietHistory {
    fn default() -> Self {
        Self {
            from_bias_by_side: zeroed_box(),
            to_bias_by_side: zeroed_box(),
            side_bias: [0; 2],
            weight_no_threat: [0; 2],
            weight_from_threat: [0; 2],
            weight_to_threat: [0; 2],
            weight_both_threat: [0; 2],
        }
    }
}

impl QuietHistory {
    const MAX_FROM_BIAS: i32 = 820;
    const MAX_TO_BIAS: i32 = 820;
    const MAX_SIDE_BIAS: i32 = 300;
    const MAX_WEIGHT_THREAT_CASE: i32 = 6029;
    const MAX_OUTPUT: i32 = 7969;
    const NON_THREAT_DENOMINATOR: i32 = 20;
    const NON_THREAT_FROM_NUMERATOR: i32 = 9;
    const NON_THREAT_TO_NUMERATOR: i32 = 9;
    const NON_THREAT_SIDE_BIAS_NUMERATOR: i32 = 2;

    pub fn get(&self, threats: Bitboard, side_to_move: Color, mv: Move) -> i32 {
        let side = side_to_move as usize;
        debug_assert!(side < 2);

        let from_is_threatened = threats.contains(mv.from());
        let to_is_threatened = threats.contains(mv.to());

        let mut value = 0i32;

        value += self.from_bias_by_side[side][mv.from()] as i32;
        value += self.to_bias_by_side[side][mv.to()] as i32;
        value += self.side_bias[side] as i32;

        let threat_term = match (from_is_threatened, to_is_threatened) {
            (false, false) => self.weight_no_threat[side],
            (true, false) => self.weight_from_threat[side],
            (false, true) => self.weight_to_threat[side],
            (true, true) => self.weight_both_threat[side],
        } as i32;

        value += threat_term;
        value.clamp(-Self::MAX_OUTPUT, Self::MAX_OUTPUT)
    }

    pub fn update(&mut self, threats: Bitboard, side_to_move: Color, mv: Move, bonus: i32) {
        let side = side_to_move as usize;

        let from_sq = mv.from();
        let to_sq = mv.to();

        let from_is_threatened = threats.contains(from_sq);
        let to_is_threatened = threats.contains(to_sq);

        apply_bonus::<{ Self::MAX_FROM_BIAS }>(
            &mut self.from_bias_by_side[side][from_sq],
            (bonus * Self::NON_THREAT_FROM_NUMERATOR) / Self::NON_THREAT_DENOMINATOR,
        );
        apply_bonus::<{ Self::MAX_TO_BIAS }>(
            &mut self.to_bias_by_side[side][to_sq],
            (bonus * Self::NON_THREAT_TO_NUMERATOR) / Self::NON_THREAT_DENOMINATOR,
        );
        apply_bonus::<{ Self::MAX_SIDE_BIAS }>(
            &mut self.side_bias[side],
            (bonus * Self::NON_THREAT_SIDE_BIAS_NUMERATOR) / Self::NON_THREAT_DENOMINATOR,
        );

        match (from_is_threatened, to_is_threatened) {
            (false, false) => {
                apply_bonus::<{ Self::MAX_WEIGHT_THREAT_CASE }>(&mut self.weight_no_threat[side], bonus);
            }
            (true, false) => {
                apply_bonus::<{ Self::MAX_WEIGHT_THREAT_CASE }>(&mut self.weight_from_threat[side], bonus);
            }
            (false, true) => {
                apply_bonus::<{ Self::MAX_WEIGHT_THREAT_CASE }>(&mut self.weight_to_threat[side], bonus);
            }
            (true, true) => {
                apply_bonus::<{ Self::MAX_WEIGHT_THREAT_CASE }>(&mut self.weight_both_threat[side], bonus);
            }
        }
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
        let threated = threats.contains(sq) as usize;
        self.buckets[captured][threated]
    }

    pub fn update_factorizer(&mut self, bonus: i32) {
        let entry = &mut self.factorizer;
        apply_bonus::<{ Self::MAX_FACTORIZER }>(entry, bonus);
    }

    pub fn update_bucket(&mut self, threats: Bitboard, sq: Square, captured: PieceType, bonus: i32) {
        let threated = threats.contains(sq) as usize;
        let entry = &mut self.buckets[captured][threated];
        apply_bonus::<{ Self::MAX_BUCKET }>(entry, bonus);
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
    const MAX_HISTORY: i32 = 14734;

    const SIZE: usize = 16384;
    const MASK: usize = Self::SIZE - 1;

    pub fn get(&self, stm: Color, key: u64) -> i32 {
        (self.entries[stm][key as usize & Self::MASK] as i32) / 82
    }

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

    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq] as i32) / 100
    }

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

    pub fn subtable_ptr(&mut self, piece: Piece, sq: Square) -> *mut PieceToHistory<i16> {
        self.entries[piece][sq].as_mut_ptr().cast()
    }

    pub fn get(&self, subtable_ptr: *mut PieceToHistory<i16>, piece: Piece, sq: Square) -> i32 {
        (unsafe { &*subtable_ptr }[piece][sq]) as i32
    }

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
