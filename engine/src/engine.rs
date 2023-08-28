use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use game::Board;
use search::{self, Cache, IterativeSearch, SearchThread, TimeControl};

use crate::commands::{OptionUciCommand, UciCommand};
use crate::perft::run_perft;

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
            cache: Default::default(),
            terminator: Default::default(),
        }
    }

    /// Executes `UciCommand` for this `Engine`.
    pub fn execute(&mut self, command: UciCommand) {
        match command {
            UciCommand::Info => {
                println!("id name Reckless 0.1.1-alpha");
                println!(
                    "option name Hash type spin default {} min {} max {}",
                    Cache::DEFAULT_SIZE,
                    Cache::MIN_SIZE,
                    Cache::MAX_SIZE
                );
                println!("option name ClearHash type button");
                println!("uciok");
            }
            UciCommand::IsReady => {
                println!("readyok");
            }

            UciCommand::NewGame => self.reset(),
            UciCommand::Position { fen, moves } => self.set_position(fen, moves),
            UciCommand::Search { time_control } => self.search(time_control),
            UciCommand::Option { option } => self.set_option(option),

            UciCommand::Stop | UciCommand::Quit => self.write_terminator(true),

            // Non-UCI commands
            UciCommand::Eval => self.evaluate(),
            UciCommand::Perft { depth } => self.perft(depth),
        }
    }

    fn set_option(&mut self, option: OptionUciCommand) {
        match option {
            OptionUciCommand::Hash(size) => self.set_cache_size(size),
            OptionUciCommand::ClearHash => self.cache.lock().unwrap().clear(),
        }
    }

    /// Sets the size of the transposition table, clearing it in the process.
    ///
    /// This function will clamp the size to the range of `Cache::MIN_SIZE` to `Cache::MAX_SIZE`.
    fn set_cache_size(&mut self, megabytes: usize) {
        let size = megabytes.min(Cache::MAX_SIZE).max(Cache::MIN_SIZE);
        self.cache = Arc::new(Mutex::new(Cache::new(size)));
    }

    /// Sets the position of this `Engine`.
    fn set_position(&mut self, fen: String, moves: Vec<&str>) {
        self.board = Board::new(&fen);
        for uci_move in moves {
            self.make_uci_move(uci_move);
        }
    }

    /// Makes the specified UCI move on the board.
    fn make_uci_move(&mut self, uci_move: &str) {
        let moves = self.board.generate_moves();
        if let Some(mv) = moves.into_iter().find(|mv| mv.to_string() == uci_move) {
            self.board.make_move(mv).expect("Invalid move")
        }
    }

    /// Resets the `Engine` to its original state.
    fn reset(&mut self) {
        self.board = Board::starting_position();
        self.write_terminator(false);
        self.cache.lock().unwrap().clear();
    }

    /// Sets the state of the terminator. If set to `true`, the current search will
    /// be stopped as soon as possible.
    fn write_terminator(&mut self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed);
    }

    /// Runs an iterative deepening search on a separate thread.
    fn search(&mut self, time_control: TimeControl) {
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
    fn perft(&mut self, depth: usize) {
        run_perft(depth, &mut self.board);
    }

    /// Statically evaluates the current position and sends a UCI report.
    fn evaluate(&self) {
        println!("{}", evaluation::evaluate_debug(&self.board));
    }
}
