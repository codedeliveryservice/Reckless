use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    board::Board,
    tables::{History, KillerMoves, NodeTable, PrincipleVariationTable, TranspositionTable},
    timeman::{Limits, TimeManager},
    types::MAX_PLY,
};

use super::{counter::NodeCounter, SearchResult, ABORT_SIGNAL, NODES_GLOBAL};

pub struct SearchThread<'a> {
    pub time_manager: TimeManager,
    pub board: &'a mut Board,
    pub history: &'a mut History,
    pub tt: &'a TranspositionTable,
    pub killers: KillerMoves,
    pub pv_table: PrincipleVariationTable,
    pub node_table: NodeTable,
    pub eval_stack: [i32; MAX_PLY],
    pub finished_depth: i32,
    pub sel_depth: usize,
    pub stopped: bool,
    pub silent: bool,
    pub nodes: NodeCounter<'a>,
    pub abort_signal: &'a AtomicBool,
}

impl<'a> SearchThread<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut History, tt: &'a TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            board,
            history,
            tt,
            killers: KillerMoves::default(),
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
