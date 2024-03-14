use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
};

use self::counter::NodeCounter;
use crate::{
    board::Board,
    tables::{CounterMoves, History, KillerMoves, NodeTable, PrincipleVariationTable, TranspositionTable},
    timeman::{Limits, TimeManager},
    types::{Move, MAX_PLY},
};

mod alphabeta;
mod aspiration;
mod counter;
mod deepening;
mod ordering;
mod quiescence;
mod selectivity;

static NODES_GLOBAL: AtomicU64 = AtomicU64::new(0);
static ABORT_SIGNAL: AtomicBool = AtomicBool::new(false);

pub struct Options {
    pub silent: bool,
    pub threads: usize,
}

pub fn start(options: Options, limits: Limits, board: &mut Board, history: &mut History, tt: &TranspositionTable) -> SearchResult {
    NODES_GLOBAL.store(0, Ordering::Relaxed);
    ABORT_SIGNAL.store(false, Ordering::Relaxed);

    thread::scope(|scope| {
        let mut threads = Vec::new();

        for _ in 0..(options.threads - 1) {
            let mut board = board.clone();
            let mut history = history.clone();

            let thread = scope.spawn(move || {
                let mut searcher = Searcher::new(Limits::Infinite, &mut board, &mut history, tt);
                searcher.silent = true;
                searcher.run()
            });

            threads.push(thread);
        }

        let mut searcher = Searcher::new(limits, board, history, tt);
        searcher.silent = options.silent;

        let result = searcher.run();

        ABORT_SIGNAL.store(true, Ordering::Relaxed);
        for thread in threads {
            thread.join().unwrap();
        }

        result
    })
}

#[derive(Clone, Copy, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: i32,
    pub nodes: u64,
}

struct Searcher<'a> {
    time_manager: TimeManager,
    board: &'a mut Board,
    history: &'a mut History,
    tt: &'a TranspositionTable,
    killers: KillerMoves,
    counters: CounterMoves,
    pv_table: PrincipleVariationTable,
    node_table: NodeTable,
    eval_stack: [i32; MAX_PLY],
    finished_depth: i32,
    sel_depth: usize,
    stopped: bool,
    silent: bool,
    nodes: NodeCounter<'a>,
    abort_signal: &'a AtomicBool,
}

impl<'a> Searcher<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut History, tt: &'a TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            board,
            history,
            tt,
            killers: KillerMoves::default(),
            counters: CounterMoves::default(),
            pv_table: PrincipleVariationTable::default(),
            node_table: NodeTable::default(),
            eval_stack: [Default::default(); MAX_PLY],
            finished_depth: Default::default(),
            sel_depth: Default::default(),
            stopped: Default::default(),
            silent: Default::default(),
            nodes: NodeCounter::new(&NODES_GLOBAL),
            abort_signal: &ABORT_SIGNAL,
        }
    }

    pub fn load_abort_signal(&self) -> bool {
        self.abort_signal.load(Ordering::Relaxed)
    }

    /// This is the main entry point for the search.
    ///
    /// It performs an iterative deepening search, incrementally increasing
    /// the search depth and printing the `info` output at each iteration.
    ///
    /// When the search is stopped, the `bestmove` command is sent to the GUI.
    pub fn run(&mut self) -> SearchResult {
        self.board.ply = 0;
        self.iterative_deepening()
    }
}
