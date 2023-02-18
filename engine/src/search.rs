use std::time::Instant;

use game::{Board, Score};

use self::killer_moves::KillerMoves;
use crate::uci::{self, UciMessage};

mod killer_moves;
mod mvv_lva;
mod negamax;
mod ordering;
mod quiescence;

pub struct SearchThread {
    nodes: u32,
    killers: KillerMoves,
}

impl SearchThread {
    fn new() -> Self {
        Self {
            nodes: Default::default(),
            killers: KillerMoves::new(),
        }
    }
}

pub struct SearchParams<'a> {
    board: &'a mut Board,
    alpha: Score,
    beta: Score,
    depth: u32,
    ply: usize,
}

impl<'a> SearchParams<'a> {
    pub fn new(board: &'a mut Board, alpha: Score, beta: Score, depth: u32, ply: usize) -> Self {
        Self {
            board,
            alpha,
            beta,
            depth,
            ply,
        }
    }
}

pub fn search(board: &mut Board, depth: u32) {
    let mut thread = SearchThread::new();

    for current in 1..=depth {
        thread.nodes = 0;

        let now = Instant::now();

        let mut pv = vec![];
        let p = SearchParams::new(board, Score::NEGATIVE_INFINITY, Score::INFINITY, current, 0);
        let score = negamax::negamax_search(p, &mut thread, &mut pv);

        let duration = now.elapsed();

        uci::send(UciMessage::SearchReport {
            depth: current,
            score,
            duration,
            pv: pv.to_vec(),
            nodes: thread.nodes,
        });

        if current == depth {
            uci::send(UciMessage::BestMove(pv[0]));
        }
    }
}
