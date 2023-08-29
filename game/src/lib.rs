pub mod board;
pub mod core;
pub mod lookup;

pub use crate::board::*;
pub use crate::core::*;

/// The maximum number of plies that can occur in a game.
pub const MAX_GAME_PLIES: usize = 1024;
