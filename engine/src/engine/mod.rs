use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use game::{Board, Color};
use search::{self, Cache, SearchThread, TimeControl};

use crate::uci::UciCommand;

mod perft;

pub struct Engine {
    board: Board,
    cache: Arc<Mutex<Cache>>,
    terminator: Arc<RwLock<bool>>,
}

impl Engine {
    pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    /// Creates a new `Engine` with the initial position set.
    pub fn new() -> Self {
        Self {
            board: Board::new(Self::START_FEN).unwrap(),
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
            UciCommand::Perft { depth } => self.perft(depth),
            UciCommand::Position { fen, moves } => self.set_position(fen, moves),
            UciCommand::Eval => self.evaluate(),

            UciCommand::Stop | UciCommand::Quit => self.set_terminator(true),

            UciCommand::Search {
                white_time,
                black_time,
                white_inc,
                black_inc,
                moves,
                movetime,
                depth,
            } => match self.board.turn {
                Color::White => self.search(white_time, white_inc, moves, movetime, depth),
                Color::Black => self.search(black_time, black_inc, moves, movetime, depth),
            },
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
        self.board = Board::new(Self::START_FEN).unwrap();
        self.set_terminator(false);
        self.cache.lock().unwrap().clear();
    }

    /// Sets the state of the terminator. If set to `true`, the current search will
    /// be stopped as soon as possible.
    fn set_terminator(&mut self, is_set: bool) {
        *self.terminator.write().unwrap() = is_set;
    }

    /// Runs an iterative deepening search on a separate thread.
    fn search(
        &mut self,
        main: Option<u32>,
        inc: Option<u32>,
        moves: Option<u32>,
        movetime: Option<u32>,
        depth: Option<usize>,
    ) {
        self.set_terminator(false);

        let board = self.board.clone();
        let terminator = self.terminator.clone();
        let cache = self.cache.clone();

        thread::spawn(move || {
            let tc = TimeControl::generate(main, inc, moves, movetime, depth);
            let thread = SearchThread::new(tc, terminator, cache);
            search::iterative_search(board, thread);
        });
    }

    /// Runs a node enumeration performance test for the current position.
    fn perft(&mut self, depth: usize) {
        perft::run(depth, &mut self.board);
    }

    /// Statically evaluates the current position and sends a UCI report.
    fn evaluate(&self) {
        println!("evaluation {}", evaluation::evaluate(&self.board));
    }
}
