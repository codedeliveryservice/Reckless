pub mod board;
pub mod core;
pub mod lookup;

pub use crate::board::*;
pub use crate::core::*;

/// The starting position in Forsyth–Edwards notation.
pub const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

/// The maximum number of plies that can occur in a game.
pub const MAX_GAME_PLIES: usize = 1024;
