use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::tables::{HistoryMoves, KillerMoves};
use crate::timeman::{Limits, TimeManager};
use crate::{board::Board, cache::Cache, types::Move};

/// Search-related data that is shared between iterations of the iterative deepening search.
pub struct SearchThread {
    pub cache: Arc<Mutex<Cache>>,
    pub terminator: Arc<AtomicBool>,
    pub time_manager: TimeManager,
    pub killers: KillerMoves,
    pub history: HistoryMoves,
}

impl SearchThread {
    /// Creates a new `SearchThread` instance.
    pub fn new(limits: Limits, terminator: Arc<AtomicBool>, cache: Arc<Mutex<Cache>>) -> Self {
        Self {
            cache,
            terminator,
            time_manager: TimeManager::new(limits),
            killers: KillerMoves::default(),
            history: HistoryMoves::default(),
        }
    }

    /// Extracts the principal variation line from the transposition table limited to the given depth.
    pub fn get_principal_variation(&self, board: &mut Board, mut depth: usize) -> Vec<Move> {
        let mut pv_line = Vec::with_capacity(depth);

        let cache = self.cache.lock().unwrap();
        while depth != 0 {
            if let Some(entry) = cache.read(board.hash(), 0) {
                pv_line.push(entry.mv);
                board.make_move(entry.mv).unwrap();
                depth -= 1;
            } else {
                break;
            }
        }

        pv_line.iter().for_each(|_| board.undo_move());
        pv_line
    }

    /// Extracts the best move from the transposition table.
    pub fn get_best_move(&self, board: &Board) -> Option<Move> {
        self.cache.lock().unwrap().read(board.hash(), 0).map(|entry| entry.mv)
    }

    /// Returns `true` if the search has been terminated.
    pub fn get_terminator(&self) -> bool {
        self.terminator.load(Ordering::Relaxed)
    }

    /// Sets the search termination flag.
    pub fn set_terminator(&mut self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed);
    }
}
