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
    Search {
        white_time: Option<u32>,
        black_time: Option<u32>,
        white_inc: Option<u32>,
        black_inc: Option<u32>,
        moves: Option<u32>,
        movetime: Option<u32>,
        depth: Option<usize>,
    },
    Perft {
        depth: usize,
    },
    Position {
        fen: String,
        moves: Vec<String>,
    },
    Eval,
    Stop,
    Quit,
}

/// Represents a message sent from `Engine` to `GUI`.
pub enum UciMessage<'a> {
    Info,
    Ready,
    Eval(Score),
    BestMove(Move),
    SearchReport {
        pv: &'a [Move],
        depth: usize,
        score: Score,
        nodes: u32,
        duration: Duration,
    },
}