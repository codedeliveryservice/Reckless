use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use game::{Board, Move, Score};

use self::history_moves::HistoryMoves;
use self::killer_moves::KillerMoves;

mod alphabeta;
mod history_moves;
mod killer_moves;
mod ordering;
mod quiescence;

pub mod cache;
pub mod iterative;
pub mod time_control;

pub use cache::*;
pub use iterative::*;
pub use time_control::*;

pub struct SearchThread {
    tc: TimeControl,
    terminator: Arc<AtomicBool>,
    cache: Arc<Mutex<Cache>>,
    start_time: Instant,
    nodes: u32,
    current_depth: usize,
    killers: KillerMoves,
    history: HistoryMoves,
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
            killers: Default::default(),
            history: Default::default(),
        }
    }

    #[inline(always)]
    fn extract_pv_line(&self, board: &mut Board, depth: usize, pv: &mut Vec<Move>) {
        if depth == 0 {
            return;
        }

        // Recursively fill the vector by going through the chain of moves in the TT
        if let Some(mv) = self.extract_pv_move(board) {
            pv.push(mv);
            board.make_move(mv).unwrap();
            self.extract_pv_line(board, depth - 1, pv);
            board.undo_move();
        }
    }

    #[inline(always)]
    fn extract_pv_move(&self, board: &Board) -> Option<Move> {
        let entry = self.cache.lock().unwrap().read(board.hash);
        entry.map(|e| e.best)
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

pub struct SearchParams {
    alpha: Score,
    beta: Score,
    depth: usize,
    ply: usize,
    allow_nmp: bool,
}

impl SearchParams {
    pub fn new(alpha: Score, beta: Score, depth: usize, ply: usize) -> Self {
        Self {
            alpha,
            beta,
            depth,
            ply,
            allow_nmp: true,
        }
    }
}
