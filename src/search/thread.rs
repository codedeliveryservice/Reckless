use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::tables::{HistoryMoves, KillerMoves};
use crate::timeman::{Limits, TimeManager};
use crate::{board::Board, cache::Cache, types::Move};

pub struct SearchThread {
    pub time_manager: TimeManager,
    pub terminator: Arc<AtomicBool>,
    pub cache: Arc<Mutex<Cache>>,
    pub killers: KillerMoves,
    pub history: HistoryMoves,
    pub nodes: u32,
    pub current_depth: usize,
}

impl SearchThread {
    pub fn new(limits: Limits, terminator: Arc<AtomicBool>, cache: Arc<Mutex<Cache>>) -> Self {
        Self {
            time_manager: TimeManager::new(limits),
            terminator,
            cache,
            killers: KillerMoves::default(),
            history: HistoryMoves::default(),
            nodes: Default::default(),
            current_depth: Default::default(),
        }
    }

    /// Extract the principal variation line from the transposition table limited to the given depth.
    pub fn get_principal_variation(&self, board: &mut Board, depth: usize) -> Vec<Move> {
        let mut pv_line = Vec::with_capacity(depth);
        let mut current_depth = depth;

        let cache = self.cache.lock().unwrap();
        while let Some(entry) = cache.read(board.hash(), 0) {
            pv_line.push(entry.mv);
            board.make_move(entry.mv).unwrap();

            current_depth -= 1;
            if current_depth == 0 {
                break;
            }
        }

        pv_line.iter().for_each(|_| board.undo_move());
        pv_line
    }

    pub fn get_terminator(&self) -> bool {
        self.terminator.load(Ordering::Relaxed)
    }

    pub fn set_terminator(&mut self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed);
    }
}
