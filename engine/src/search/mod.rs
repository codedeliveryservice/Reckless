use std::sync::{Arc, RwLock};
use std::time::Instant;

use game::{Board, Move, Score};

use self::killer_moves::KillerMoves;
use crate::uci::{self, UciMessage};

mod killer_moves;
mod mvv_lva;
mod negamax;
mod ordering;
mod quiescence;

pub mod time_control;
pub use time_control::*;

pub struct SearchThread {
    tc: TimeControl,
    terminator: Arc<RwLock<bool>>,
    nodes: u32,
    killers: KillerMoves,
    pv_length: [usize; 64],
    pv_table: [[Move; 64]; 64],
}

impl SearchThread {
    pub fn new(tc: TimeControl, terminator: Arc<RwLock<bool>>) -> Self {
        Self {
            tc,
            terminator,
            nodes: Default::default(),
            killers: KillerMoves::new(),
            pv_length: [Default::default(); 64],
            pv_table: [[Default::default(); 64]; 64],
        }
    }

    #[inline(always)]
    pub fn update_pv(&mut self, ply: usize, mv: Move) {
        self.pv_table[ply][ply] = mv;
        for index in (ply + 1)..self.pv_length[ply + 1] {
            self.pv_table[ply][index] = self.pv_table[ply + 1][index];
        }
        self.pv_length[ply] = self.pv_length[ply + 1];
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

pub fn search(board: &mut Board, mut thread: SearchThread) {
    let mut last_best = Default::default();

    for depth in 1..=thread.tc.max_depth {
        thread.nodes = 0;

        let now = Instant::now();
        let params = SearchParams::new(board, Score::NEGATIVE_INFINITY, Score::INFINITY, depth, 0);
        let score = negamax::negamax_search(params, &mut thread);
        let duration = now.elapsed();

        if thread.tc.is_time_over() || thread.requested_termination() {
            uci::send(UciMessage::BestMove(last_best));
            return;
        }

        last_best = thread.pv_table[0][0];

        uci::send(UciMessage::SearchReport {
            depth,
            score,
            duration,
            pv: &thread.pv_table[0][..thread.pv_length[0]],
            nodes: thread.nodes,
        });
    }

    uci::send(UciMessage::BestMove(last_best));
}
