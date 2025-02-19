pub mod bitboard;
pub mod castling;
pub mod color;
pub mod movelist;
pub mod moves;
pub mod piece;
pub mod score;
pub mod square;

pub use bitboard::*;
pub use castling::*;
pub use color::*;
pub use movelist::*;
pub use moves::*;
pub use piece::*;
pub use score::*;
pub use square::*;

/// The maximum number of plies that can be searched.
pub const MAX_PLY: usize = 128;

/// According to [Chess Programming Wiki](https://www.chessprogramming.org/Encoding_Moves#MoveIndex),
/// the maximum number of chess moves in a certain position *appears* to be 218.
pub const MAX_MOVES: usize = 218;

#[rustfmt::skip]
pub enum Rank { R1, R2, R3, R4, R5, R6, R7, R8 }
