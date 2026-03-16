pub mod arrayvec;
pub mod bitboard;
pub mod castling;
pub mod color;
pub mod movelist;
pub mod moves;
pub mod piece;
pub mod score;
pub mod square;
pub mod zobrist;

pub use arrayvec::*;
pub use bitboard::*;
pub use castling::*;
pub use color::*;
pub use movelist::*;
pub use moves::*;
pub use piece::*;
pub use score::*;
pub use square::*;
pub use zobrist::*;

/// The maximum number of plies that can be searched.
pub const MAX_PLY: usize = 128;

/// The maximum number of chess moves in any legal position is 218.
/// For more details see https://lichess.org/@/Tobs40/blog/why-a-position-cant-have-more-than-218-moves/a5xdxeqs
/// Padding added because an optimization in MoveList::push_setwise writes 16 moves at a time to the move list (218 + 16 < 256).
pub const MAX_MOVES: usize = 256;

#[rustfmt::skip]
#[repr(u8)]
#[derive(PartialEq, PartialOrd)]
pub enum Rank { R1, R2, R3, R4, R5, R6, R7, R8 }

#[rustfmt::skip]
#[repr(u8)]
#[derive(PartialEq, PartialOrd)]
pub enum File { A, B, C, D, E, F, G, H }

impl File {
    pub fn is_kingside(&self) -> bool {
        *self >= File::E
    }
}
