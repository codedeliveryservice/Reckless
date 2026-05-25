/// Represents the sets of random numbers used to produce an *almost* unique hash value
/// for a position using [Zobrist Hashing](https://en.wikipedia.org/wiki/Zobrist_hashing)
/// generated using the SplitMix64 pseudorandom number generator.
pub struct Zobrist {
    pub pieces: [[u64; 64]; 12],
    pub en_passant: [u64; 64],
    pub castling: [u64; 16],
    pub side: u64,
    pub halfmove_clock: [u64; 16],
}

pub const ZOBRIST: Zobrist = {
    const SEED: u64 = 0xFFAA_B58C_5833_FE89u64;
    const INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;

    let mut zobrist = [0; 865];
    let mut state = SEED;

    let mut i = 0;
    while i < zobrist.len() {
        state = state.wrapping_add(INCREMENT);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        zobrist[i] = z ^ (z >> 31);

        i += 1;
    }
    unsafe { std::mem::transmute(zobrist) }
};
