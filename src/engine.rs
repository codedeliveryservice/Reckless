use crate::tools::perft;
use crate::{board::Board, cache::Cache, search::Searcher, timeman::Limits};

pub struct Engine {
    pub board: Board,
    pub cache: Cache,
}

impl Engine {
    /// Creates a new `Engine` with the initial position set.
    pub fn new() -> Self {
        Self {
            board: Board::starting_position(),
            cache: Cache::default(),
        }
    }

    /// Clears the transposition table.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Sets the size of the transposition table, clearing it in the process.
    pub fn set_cache_size(&mut self, megabytes: usize) {
        self.cache = Cache::new(megabytes);
    }

    /// Makes the specified UCI move on the board.
    pub fn make_uci_move(&mut self, uci_move: &str) {
        let moves = self.board.generate_moves();
        if let Some(mv) = moves.iter().find(|mv| mv.to_string() == uci_move) {
            self.board.make_move(mv).expect("UCI move should be legal");
        }
    }

    /// Resets the `Engine` to its original state.
    pub fn reset(&mut self) {
        self.cache.clear();
        self.board = Board::starting_position();
    }

    /// Runs an iterative deepening search on a separate thread.
    pub fn search(&mut self, limits: Limits) {
        let board = self.board.clone();
        Searcher::new(board, limits, &mut self.cache).iterative_deepening();
    }

    /// Runs a node enumeration performance test for the current position.
    pub fn perft(&mut self, depth: usize) {
        perft(depth, &mut self.board);
    }
}
