//! The Universal Chess Interface (UCI) is an open communication protocol that enables
//! chess engines to communicate with other programs including Graphical User Interfaces.
//!
//! See [UCI](https://www.chessprogramming.org/UCI) for more information.

pub mod parser;
pub mod sender;

pub use parser::*;
pub use sender::*;

use game::{Move, Score};
use std::time::Duration;

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

/// Represents a message sent from `Engine` to `GUI`.
pub enum UciMessage {
    Info,
    Ready,
    Eval(Score),
    BestMove(Move),
    SearchReport {
        pv: Vec<Move>,
        depth: u32,
        score: Score,
        nodes: u32,
        duration: Duration,
    },
}
