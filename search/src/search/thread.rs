use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use game::{Board, Move};

use crate::{Cache, TimeControl};

pub struct SearchThread {
    pub tc: TimeControl,
    pub terminator: Arc<AtomicBool>,
    pub cache: Arc<Mutex<Cache>>,
    pub start_time: Instant,
    pub nodes: u32,
    pub current_depth: usize,
}

impl SearchThread {
    pub fn new(tc: TimeControl, terminator: Arc<AtomicBool>, cache: Arc<Mutex<Cache>>) -> Self {
        Self {
            tc,
            terminator,
            cache,
            start_time: Instant::now(),
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
            pv_line.push(entry.best);
            board.make_move(entry.best).unwrap();

            current_depth -= 1;
            if current_depth == 0 {
                break;
            }
        }

        pv_line.iter().for_each(|_| board.undo_move());
        pv_line
    }

    #[inline(always)]
    pub fn get_terminator(&self) -> bool {
        self.terminator.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn set_terminator(&mut self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn is_time_over(&self) -> bool {
        self.tc.is_time_over(self.start_time.elapsed())
    }
}
