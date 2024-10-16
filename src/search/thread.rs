use super::{counter::AtomicCounter, ABORT_SIGNAL, NODES_GLOBAL};
use crate::{
    board::Board,
    parameters::Parameters,
    tables::{History, NodeTable, PrincipalVariationTable, TranspositionTable},
    time::{Limits, TimeManager},
    types::{Move, MAX_PLY},
};

pub struct SearchThread<'a> {
    /// The time manager instance that controls the search time.
    pub time_manager: TimeManager,
    /// Flag for a quick check if the search has been stopped.
    /// This is set when the time manager has run out of time,
    /// or the main thread has sent an abort signal.
    pub stopped: bool,

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
    /// A table for storing the principal variation line.
    pub pv_table: PrincipalVariationTable,
    /// A table for storing the number of nodes searched at root the for each move.
    pub node_table: NodeTable,
    pub params: Parameters,

    pub ply: usize,
    /// The depth of the last completed search.
    pub finished_depth: i32,
    /// The maximum depth reached in the current search, including qsearch.
    pub sel_depth: usize,

    /// Atomic counter for multi-threaded node counting.
    pub nodes: AtomicCounter<'a>,
}

impl<'a> SearchThread<'a> {
    /// Creates a new search thread instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut History, tt: &'a TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(&ABORT_SIGNAL, limits),
            stopped: false,
            board,
            history,
            tt,
            killers: [Move::NULL; MAX_PLY],
            eval_stack: [0; MAX_PLY],
            pv_table: PrincipalVariationTable::default(),
            node_table: NodeTable::default(),
            params: Parameters::default(),
            ply: 0,
            finished_depth: 0,
            sel_depth: 0,
            nodes: AtomicCounter::new(&NODES_GLOBAL),
        }
    }

    pub fn apply_null_move(&mut self) {
        self.ply += 1;
        self.board.make_null_move();
    }

    pub fn revert_null_move(&mut self) {
        self.ply -= 1;
        self.board.undo_null_move();
    }

    pub fn apply_move(&mut self, mv: Move) -> bool {
        self.ply += 1;
        let is_legal = self.board.make_move::<true, false>(mv);
        if !is_legal {
            self.revert_move();
        }
        is_legal
    }

    pub fn revert_move(&mut self) {
        self.ply -= 1;
        self.board.undo_move::<true>();
    }
}
