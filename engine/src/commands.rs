//! The Universal Chess Interface (UCI) is an open communication protocol that enables
//! chess engines to communicate with other programs including Graphical User Interfaces.
//!
//! See [UCI](https://www.chessprogramming.org/UCI) for more information.
/// Represents a command sent from `GUI` to `Engine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UciCommand<'a> {
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
        moves: Vec<&'a str>,
    },
    Eval,
    Stop,
    Quit,
}
