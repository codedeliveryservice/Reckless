use crate::{
    board::Board,
    tables::{History, NodeTable, PrincipleVariationTable, TranspositionTable},
    time::{Limits, TimeManager},
    types::{Move, MAX_PLY},
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
    pub board: &'a mut Board,
    /// Persistent between searches history table for move ordering.
    pub history: &'a mut History,
    /// Hash table with interior mutability for shared memory parallelism.
    pub tt: &'a TranspositionTable,

    /// Moves that caused a beta cutoff in a sibling node at each ply.
    pub killers: [Move; MAX_PLY],
    /// A stack for storing the static evaluation of the position at each ply.
    pub eval_stack: [i32; MAX_PLY],
    /// A table for storing the principle variation line.
    pub pv_table: PrincipleVariationTable,
    /// A table for storing the number of nodes searched at root the for each move.
    pub node_table: NodeTable,

    /// The depth of the last completed search.
    pub finished_depth: i32,
    /// The maximum depth reached in the current search, including qsearch.
    pub sel_depth: usize,

    /// Atomic counter for multi-threaded node counting.
    pub nodes: NodeCounter<'a>,
}

impl<'a> SearchThread<'a> {
    /// Creates a new search thread instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut History, tt: &'a TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(&ABORT_SIGNAL, limits),
            stopped: false,
            silent: false,
            board,
            history,
            tt,
            killers: [Move::NULL; MAX_PLY],
            eval_stack: [0; MAX_PLY],
            pv_table: PrincipleVariationTable::default(),
            node_table: NodeTable::default(),
            finished_depth: 0,
            sel_depth: 0,
            nodes: NodeCounter::new(&NODES_GLOBAL),
        }
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
