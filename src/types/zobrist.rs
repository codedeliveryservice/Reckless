use crate::types::{Color, PieceType};

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

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZobristInfo {
    pub full: u64,
    pub pawn: u64,
    pub non_pawn_white: u64,
    pub non_pawn_black: u64,
    pub major: u64,
    pub minor: u64,
}

impl ZobristInfo {
    pub const fn zero() -> Self {
        Self {
            full: 0,
            pawn: 0,
            non_pawn_white: 0,
            non_pawn_black: 0,
            major: 0,
            minor: 0,
        }
    }

    pub fn xor_assign(&mut self, other: &Self) {
        self.full ^= other.full;
        self.pawn ^= other.pawn;
        self.non_pawn_white ^= other.non_pawn_white;
        self.non_pawn_black ^= other.non_pawn_black;
        self.major ^= other.major;
        self.minor ^= other.minor;
    }

    pub fn update_fullkey(&mut self, value: u64) {
        self.full ^= value;
    }

    pub fn toggle(&mut self, color: Color, ptype: PieceType, sq: usize) {
        let piece_key = ZOBRIST.pieces[ptype as usize * 2 + color as usize][sq];
        
        self.full ^= piece_key;
        
        match ptype {
            PieceType::Pawn => {
                self.pawn ^= piece_key;
            }
            _ => {
                match color {
                    Color::White => self.non_pawn_white ^= piece_key,
                    Color::Black => self.non_pawn_black ^= piece_key,
                }
                match ptype {
                    PieceType::Rook | PieceType::Queen | PieceType::King => {
                        self.major ^= piece_key;
                    }
                    _ => {}
                }
                match ptype {
                    PieceType::Knight | PieceType::Bishop | PieceType::King => {
                        self.minor ^= piece_key;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn full_key(&self) -> u64 {
        self.full
    }

    pub const fn pawn_key(&self) -> u64 {
        self.pawn
    }

    pub const fn non_pawn_key(&self, color: Color) -> u64 {
        match color {
            Color::White => self.non_pawn_white,
            Color::Black => self.non_pawn_black,
        }
    }

    pub const fn major_key(&self) -> u64 {
        self.major
    }

    pub const fn minor_key(&self) -> u64 {
        self.minor
    }
}
