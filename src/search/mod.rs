use crate::{
    board::Board,
    tables::{ContinuationHistory, CounterMoves, HistoryMoves, KillerMoves, NodeTable, PrincipleVariationTable, TranspositionTable},
    timeman::{Limits, TimeManager},
    types::{Move, MAX_PLY},
};

mod alphabeta;
mod aspiration;
mod deepening;
mod ordering;
mod quiescence;
mod selectivity;

#[derive(Clone, Copy, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: i32,
}

pub struct Searcher<'a> {
    time_manager: TimeManager,
    board: &'a mut Board,
    history: &'a mut HistoryMoves,
    tt: &'a mut TranspositionTable,
    killers: KillerMoves,
    counters: CounterMoves,
    followup_history: ContinuationHistory,
    pv_table: PrincipleVariationTable,
    node_table: NodeTable,
    eval_stack: [i32; MAX_PLY],
    finished_depth: i32,
    sel_depth: usize,
    stopped: bool,
    nodes: u64,
    silent: bool,
}

impl<'a> Searcher<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut HistoryMoves, tt: &'a mut TranspositionTable) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            board,
            history,
            tt,
            killers: KillerMoves::default(),
            counters: CounterMoves::default(),
            followup_history: ContinuationHistory::default(),
            pv_table: PrincipleVariationTable::default(),
            node_table: NodeTable::default(),
            eval_stack: [Default::default(); MAX_PLY],
            finished_depth: Default::default(),
            sel_depth: Default::default(),
            stopped: Default::default(),
            nodes: Default::default(),
            silent: Default::default(),
        }
    }

    /// Returns the number of nodes searched.
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Controls whether the search should be silent. Defaults to `false`.
    pub fn silent(&mut self, silent: bool) {
        self.silent = silent;
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
