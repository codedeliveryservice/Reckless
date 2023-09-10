mod macros;

pub mod bitboard;
pub mod castling;
pub mod color;
pub mod move_list;
pub mod moves;
pub mod piece;
pub mod square;

pub use bitboard::*;
pub use castling::*;
pub use color::*;
pub use move_list::*;
pub use moves::*;
pub use piece::*;
pub use square::*;

/// The maximum number of plies that can occur in a game.
pub const MAX_GAME_PLIES: usize = 1024;

/// The maximum number of plies that can be searched.
pub const MAX_SEARCH_DEPTH: usize = 64;
