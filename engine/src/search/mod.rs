use std::sync::{Arc, Mutex, RwLock};

use game::{Board, Move, Score};

use self::killer_moves::KillerMoves;

mod killer_moves;
mod negamax;
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
    terminator: Arc<RwLock<bool>>,
    cache: Arc<Mutex<Cache>>,
    nodes: u32,
    killers: KillerMoves,
}

impl SearchThread {
    pub fn new(tc: TimeControl, terminator: Arc<RwLock<bool>>, cache: Arc<Mutex<Cache>>) -> Self {
        Self {
            tc,
            terminator,
            cache,
            nodes: Default::default(),
            killers: KillerMoves::new(),
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
            board.take_back();
        }
    }

    #[inline(always)]
    fn extract_pv_move(&self, board: &Board) -> Option<Move> {
        let entry = self.cache.lock().unwrap().read(board.hash_key);
        entry.map(|e| e.best)
    }

    #[inline(always)]
    pub fn requested_termination(&self) -> bool {
        *self.terminator.read().unwrap()
    }

    #[inline(always)]
    pub fn check_on(&self) -> bool {
        self.nodes % 4096 == 0 && (self.tc.is_time_over() || self.requested_termination())
    }
}

pub struct SearchParams<'a> {
    board: &'a mut Board,
    alpha: Score,
    beta: Score,
    depth: usize,
    ply: usize,
}

impl<'a> SearchParams<'a> {
    pub const MAX_PLY: usize = 64;

    pub fn new(board: &'a mut Board, alpha: Score, beta: Score, depth: usize, ply: usize) -> Self {
        Self {
            board,
            alpha,
            beta,
            depth,
            ply,
        }
    }
}
