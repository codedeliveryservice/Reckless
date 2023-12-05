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
    board: Board,
    time_manager: TimeManager,
    cache: &'a mut Cache,
    history: &'a mut HistoryMoves,
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
    pub fn new(board: Board, limits: Limits, history: &'a mut HistoryMoves, cache: &'a mut Cache) -> Self {
        Self {
            board,
            time_manager: TimeManager::new(limits),
            cache,
            history,
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
    pub fn nodes(&self) -> u32 {
        self.nodes
    }

    /// Controls whether the search should be silent. Defaults to `false`.
    pub fn silent(&mut self, silent: bool) {
        self.silent = silent;
    }
}
