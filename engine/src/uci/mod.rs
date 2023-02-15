//! The Universal Chess Interface (UCI) is an open communication protocol that enables
//! chess engines to communicate with other programs including Graphical User Interfaces.
//!
//! See [UCI](https://www.chessprogramming.org/UCI) for more information.

pub mod parser;

pub use parser::*;

/// Represents a command sent from `GUI` to `Engine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UciCommand {
    Info,
    IsReady,
    NewGame,
    Search { depth: u32 },
    Perft { depth: u32 },
    Position { fen: String, moves: Vec<String> },
    Eval,
    Stop,
    Quit,
}
