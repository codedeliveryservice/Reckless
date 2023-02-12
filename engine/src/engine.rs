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

        self.board.make_move(mv).unwrap();
    }

    pub fn search(&mut self, depth: u32) {
        search::search(&mut self.board, depth, |result| {
            let nps = result.nodes as f32 / result.time.as_secs_f32();
            let ms = result.time.as_millis();

            print!(
                "info depth {} score cp {} nodes {} time {} nps {:.0} pv",
                result.depth, result.score, result.nodes, ms, nps
            );

            for mv in &result.pv {
                print!(" {}", mv);
            }
            println!();

            if result.depth == depth {
                println!("bestmove {}", result.pv[0]);
            }
        });
    }

    pub fn eval(&self) {
        println!("evaluation {}", evaluation::evaluate(&self.board));
    }

    pub fn perft(&mut self, depth: u32) {
        crate::perft::run(depth, &mut self.board);
    }
}
