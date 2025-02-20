/// Represents the sets of random numbers used to produce an *almost* unique hash value
/// for a position using [Zobrist Hashing](https://en.wikipedia.org/wiki/Zobrist_hashing).
pub struct Zobrist {
    pub pieces: [[u64; 64]; 12],
    pub en_passant: [u64; 64],
    pub castling: [u64; 16],
    pub side: u64,
}

pub const ZOBRIST: Zobrist = {
    let mut seed = 0xFFAA_B58C_5833_FE89u64;
    let mut zobrist = [0; 849];

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
