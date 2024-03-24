use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    board::Board,
    tables::{History, KillerMoves, NodeTable, PrincipleVariationTable, TranspositionTable},
    timeman::{Limits, TimeManager},
    types::MAX_PLY,
};

use super::{counter::NodeCounter, SearchResult, ABORT_SIGNAL, NODES_GLOBAL};

pub struct SearchThread<'a> {
    /// The time manager instance that controls the search time.
    pub time_manager: TimeManager,
    /// Flag for a quick check if the search has been stopped.
    /// This is set when the time manager has run out of time,
    /// or the main thread has sent an abort signal.
    pub stopped: bool,
    /// Flag to suppress output during the search (`info` and `bestmove` commands).
    pub silent: bool,

    /// The board state to start the search from.
    pub board: Board,
    /// Hash table with interior mutability for shared memory parallelism.
    pub tt: &'a TranspositionTable,
    /// Persistent between searches history table for move ordering.
    pub history: &'a mut History,

    /// The killer move heuristic.
    pub killers: KillerMoves,
    /// A table for storing the principle variation line.
    pub pv_table: PrincipleVariationTable,
    /// A table for storing the number of nodes searched at root the for each move.
    pub node_table: NodeTable,
    /// A stack for storing the static evaluation of the position at each ply.
    pub eval_stack: [i32; MAX_PLY],

    /// The depth of the last completed search.
    pub finished_depth: i32,
    /// The maximum depth reached in the current search, including qsearch.
    pub sel_depth: usize,

    /// Atomic counter for multi-threaded node counting.
    pub nodes: NodeCounter<'a>,
    /// The main thread sends an abort signal to all search threads.
    pub abort_signal: &'a AtomicBool,
}

impl<'a> SearchThread<'a> {
    /// Creates a new search thread instance.
    pub fn new(limits: Limits, board: Board, history: &'a mut History, tt: &'a TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            stopped: false,
            silent: false,
            board,
            tt,
            history,
            killers: KillerMoves::default(),
            pv_table: PrincipleVariationTable::default(),
            node_table: NodeTable::default(),
            eval_stack: [0; MAX_PLY],
            finished_depth: 0,
            sel_depth: 0,
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
