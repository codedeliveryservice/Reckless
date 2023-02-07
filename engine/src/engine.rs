use game::Board;

use crate::{evaluation, search};

#[derive(Default)]
pub struct Engine {
    board: Board,
}

impl Engine {
    pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_position(&mut self, fen: &str) {
        self.board = Board::new(fen).unwrap();
    }

    /// Plays the specified UCI move.
    ///
    /// The move format is in long algebraic notation.
    ///
    /// # Examples
    ///
    /// * e2e4
    /// * e7e5
    /// * e1g1 (white short castling)
    /// * e7e8q (queen promotion)
    pub fn play_uci_move(&mut self, uci_move: &str) {
        let mv = self
            .board
            .generate_moves()
            .into_iter()
            .find(|m| m.to_string() == uci_move)
            .unwrap();

        self.board.apply_move(mv).unwrap();
    }

    pub fn search(&mut self, depth: u32) {
        let result = search::search(&mut self.board, depth);
        println!("info score cp {} depth {} nodes {}", result.score, depth, result.nodes);
        println!("bestmove {}", result.best_move);
    }

    pub fn eval(&self) {
        println!("evaluation {}", evaluation::evaluate(&self.board));
    }

    pub fn perft(&mut self, depth: u32) {
        crate::perft::run(depth, &mut self.board);
    }
}
