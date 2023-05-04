//! The Universal Chess Interface (UCI) is an open communication protocol that enables
//! chess engines to communicate with other programs including Graphical User Interfaces.
//!
//! See [UCI](https://www.chessprogramming.org/UCI) for more information.
use search::TimeControl;

/// Represents a command sent from `GUI` to `Engine`.
#[derive(Debug, PartialEq)]
pub enum UciCommand<'a> {
    Info,
    IsReady,
    NewGame,
    Option { option: OptionUciCommand },
    Search { time_control: TimeControl },
    Perft { depth: usize },
    Position { fen: String, moves: Vec<&'a str> },
    Eval,
    Stop,
    Quit,
}

#[derive(Debug, PartialEq)]
pub enum OptionUciCommand {
    Hash(usize),
    ClearHash,
}
