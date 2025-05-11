use super::Piece;

/// Represents the sets of random numbers used to produce an *almost* unique hash value
/// for a position using [Zobrist Hashing](https://en.wikipedia.org/wiki/Zobrist_hashing).
pub struct Zobrist {
    pub pieces: [[u64; 64]; 12],
    pub en_passant: [u64; 64],
    pub castling: [u64; 16],
    pub side: u64,
    pub halfmove_clock: [u64; 16],
}

pub const ZOBRIST: Zobrist = {
    let mut seed = 0xFFAA_B58C_5833_FE89u64;
    let mut zobrist = [0; 865];

    let mut i = 0;
    while i < zobrist.len() {
        // https://en.wikipedia.org/wiki/Xorshift
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;

        zobrist[i] = seed;
        i += 1;
    }
    unsafe { std::mem::transmute(zobrist) }
};

struct Prng(u64);

impl Prng {
    pub fn new(seed: u64) -> Prng {
        Prng(seed)
    }

    pub fn rand64(&mut self) -> u64 {
        (*self).0 ^= (*self).0 >> 12;
        (*self).0 ^= (*self).0 << 25;
        (*self).0 ^= (*self).0 >> 27;
        u64::wrapping_mul(self.0, 2685821657736338717)
    }
}

pub static mut PSQ: [[u64; 64]; 16] = [[0; 64]; 16];

pub fn material(pc: Piece, num: usize) -> u64 {
    const MAP: [usize; 12] = [
        1,  // WhitePawn,
        9,  // BlackPawn,
        2,  // WhiteKnight,
        10, // BlackKnight,
        3,  // WhiteBishop,
        11, // BlackBishop,
        4,  // WhiteRook,
        12, // BlackRook,
        5,  // WhiteQueen,
        13, // BlackQueen,
        6,  // WhiteKing,
        14, // BlackKing,
    ];

    unsafe { PSQ[MAP[pc as usize]][num] }
}

// position::init() initializes at startup the various arrays used to
// compute hash u64s.

pub fn init() {
    let mut rng = Prng::new(1070372);

    unsafe {
        for i in 1..15 {
            if i != 7 && i != 8 {
                for s in 0..64 {
                    PSQ[i][s] = rng.rand64();
                }
            }
        }
    }
}
