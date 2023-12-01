use self::pvtable::PrincipleVariationTable;
use crate::{
    board::Board,
    cache::Cache,
    tables::{HistoryMoves, KillerMoves},
    timeman::{Limits, TimeManager},
};

mod alphabeta;
mod aspiration;
mod deepening;
mod ordering;
mod pvtable;
mod quiescence;
mod selectivity;

pub struct Searcher<'a> {
    pub nodes: u32,
    pub stopped: bool,
    pub print_to_stdout: bool,
    board: Board,
    cache: &'a mut Cache,
    time_manager: TimeManager,
    killers: KillerMoves,
    history: &'a mut HistoryMoves,
    pv_table: PrincipleVariationTable,
    sel_depth: usize,
}

impl<'a> Searcher<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(board: Board, limits: Limits, history: &'a mut HistoryMoves, cache: &'a mut Cache) -> Self {
        Self {
            board,
            cache,
            history,
            time_manager: TimeManager::new(limits),
            killers: KillerMoves::default(),
            pv_table: Default::default(),
            sel_depth: Default::default(),
            nodes: Default::default(),
            stopped: Default::default(),
            print_to_stdout: true,
        }
    }
}
