use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use game::Board;
use search::{self, Cache, SearchThread, TimeControl};

use crate::commands::UciCommand;
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
            cache: Arc::new(Mutex::new(Cache::new(2))),
            terminator: Default::default(),
        }
    }

    /// Executes `UciCommand` for this `Engine`.
    pub fn execute(&mut self, command: UciCommand) {
        match command {
            UciCommand::Info => {
                println!("id name Reckless");
                println!("uciok");
            }
            UciCommand::IsReady => {
                println!("readyok");
            }

            UciCommand::NewGame => self.reset(),
            UciCommand::Position { fen, moves } => self.set_position(fen, moves),
            UciCommand::Search { time_control } => self.search(time_control),

            UciCommand::Stop | UciCommand::Quit => self.write_terminator(true),

            // Non-UCI commands
            UciCommand::Eval => self.evaluate(),
            UciCommand::Perft { depth } => self.perft(depth),
        }
    }

    /// Sets the position of this `Engine`.
    fn set_position(&mut self, fen: String, moves: Vec<&str>) {
        // TODO: Validate `fen`
        self.board = Board::new(&fen).unwrap();
        for uci_move in moves {
            self.make_uci_move(uci_move);
        }
    }

    /// Makes the specified UCI move on the board.
    fn make_uci_move(&mut self, uci_move: &str) {
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
            search::iterative_search(board, thread);
        });
    }

    /// Runs a node enumeration performance test for the current position.
    fn perft(&mut self, depth: usize) {
        run_perft(depth, &mut self.board);
    }

    /// Statically evaluates the current position and sends a UCI report.
    fn evaluate(&self) {
        let score = evaluation::evaluate_absolute_score(&self.board);
        println!("evaluation {}", score);
    }
}
