use self::pvtable::PrincipleVariationTable;
use crate::{
    board::Board,
    cache::Cache,
    tables::{HistoryMoves, KillerMoves},
    timeman::{Limits, TimeManager},
    types::MAX_PLY,
};

mod alphabeta;
mod aspiration;
mod deepening;
mod ordering;
mod pvtable;
mod quiescence;
mod selectivity;

pub struct Searcher<'a> {
    time_manager: TimeManager,
    board: &'a mut Board,
    history: &'a mut HistoryMoves,
    cache: &'a mut Cache,
    killers: KillerMoves,
    pv_table: PrincipleVariationTable,
    eval_stack: [i32; MAX_PLY],
    sel_depth: usize,
    stopped: bool,
    nodes: u32,
    silent: bool,
}

impl<'a> Searcher<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(limits: Limits, board: &'a mut Board, history: &'a mut HistoryMoves, cache: &'a mut Cache) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            board,
            history,
            cache,
            killers: KillerMoves::default(),
            pv_table: PrincipleVariationTable::default(),
            eval_stack: [Default::default(); MAX_PLY],
            sel_depth: Default::default(),
            stopped: Default::default(),
            nodes: Default::default(),
            silent: Default::default(),
        }
    }

    /// Returns the number of nodes searched.
    pub const fn nodes(&self) -> u32 {
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
    pub fn run(&mut self) {
        self.board.ply = 0;
        self.iterative_deepening();
    }
}
