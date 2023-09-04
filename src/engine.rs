use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::search::{IterativeSearch, SearchThread};
use crate::{board::Board, cache::Cache, evaluation, perft::run_perft, time_control::TimeControl};

pub struct Engine {
    pub board: Board,
    pub cache: Arc<Mutex<Cache>>,
    pub terminator: Arc<AtomicBool>,
}

impl Engine {
    /// Creates a new `Engine` with the initial position set.
    pub fn new() -> Self {
        Self {
            board: Board::starting_position(),
            cache: Arc::default(),
            terminator: Arc::default(),
        }
    }

    /// Clears the transposition table.
    pub fn clear_cache(&mut self) {
        self.cache.lock().unwrap().clear();
    }

    /// Sets the size of the transposition table, clearing it in the process.
    pub fn set_cache_size(&mut self, megabytes: usize) {
        self.cache = Arc::new(Mutex::new(Cache::new(megabytes)));
    }

    /// Makes the specified UCI move on the board.
    pub fn make_uci_move(&mut self, uci_move: &str) {
        let moves = self.board.generate_moves();
        if let Some(mv) = moves.iter().find(|mv| mv.to_string() == uci_move) {
            self.board.make_move(mv).expect("UCI move should be legal");
        }
    }

    /// Stops the current search as soon as possible.
    pub fn stop(&mut self) {
        self.write_terminator(true);
    }

    /// Resets the `Engine` to its original state.
    pub fn reset(&mut self) {
        self.write_terminator(false);
        self.cache.lock().unwrap().clear();
        self.board = Board::starting_position();
    }

    /// Runs an iterative deepening search on a separate thread.
    pub fn search(&mut self, time_control: TimeControl) {
        self.write_terminator(false);

        let board = self.board.clone();
        let terminator = self.terminator.clone();
        let cache = self.cache.clone();

        thread::spawn(move || {
            let thread = SearchThread::new(time_control, terminator, cache);
            IterativeSearch::new(board, thread).search();
        });
    }

    /// Runs a node enumeration performance test for the current position.
    pub fn perft(&mut self, depth: usize) {
        run_perft(depth, &mut self.board);
    }

    /// Statically evaluates the current position and sends a UCI report.
    pub fn evaluate(&self) {
        println!("{}", evaluation::evaluate_debug(&self.board));
    }

    /// Sets the state of the terminator. If set to `true`, the current search will
    /// be stopped as soon as possible.
    fn write_terminator(&mut self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed);
    }
}
