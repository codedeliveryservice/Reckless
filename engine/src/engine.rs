use game::Board;

use crate::{
    evaluation, perft, search,
    uci::{self, UciCommand, UciMessage},
};

pub struct Engine {
    board: Board,
}

impl Engine {
    pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    /// Creates a new `Engine` with the initial position set.
    pub fn new() -> Self {
        Self {
            board: Board::new(Self::START_FEN).unwrap(),
        }
    }

    /// Executes `UciCommand` for this `Engine`.
    pub fn execute(&mut self, command: UciCommand) {
        match command {
            UciCommand::Info => uci::send(UciMessage::Info),
            UciCommand::IsReady => uci::send(UciMessage::Ready),

            UciCommand::NewGame => self.reset(),
            UciCommand::Search { depth } => self.search(depth),
            UciCommand::Perft { depth } => self.perft(depth),
            UciCommand::Position { fen, moves } => self.set_position(fen, moves),
            UciCommand::Eval => uci::send(UciMessage::Eval(evaluation::evaluate(&self.board))),

            UciCommand::Stop | UciCommand::Quit => {}
        }
    }

    /// Sets the position of this `Engine`.
    fn set_position(&mut self, fen: String, moves: Vec<String>) {
        // TODO: Validate `fen`
        self.board = Board::new(&fen).unwrap();
        for uci_move in moves {
            self.make_uci_move(uci_move);
        }
    }

    /// Makes the specified UCI move on the board.
    fn make_uci_move(&mut self, uci_move: String) {
        for mv in self.board.generate_moves() {
            if mv.to_string() == uci_move {
                // TODO: Validate the legality of the move
                self.board.make_move(mv).unwrap();
                break;
            }
        }
    }

    /// Resets the `Engine` to its original state.
    fn reset(&mut self) {
        self.board = Board::new(Self::START_FEN).unwrap();
    }

    pub fn search(&mut self, depth: u32) {
        search::search(&mut self.board, depth);
    }

    pub fn perft(&mut self, depth: u32) {
        perft::run(depth, &mut self.board);
    }
}
