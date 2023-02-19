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

pub struct SearchThread {
    terminator: Arc<RwLock<bool>>,
    nodes: u32,
    killers: KillerMoves,
    pv_length: [usize; 64],
    pv_table: [[Move; 64]; 64],
}

impl SearchThread {
    fn new(terminator: Arc<RwLock<bool>>) -> Self {
        Self {
            terminator,
            nodes: Default::default(),
            killers: KillerMoves::new(),
            pv_length: [Default::default(); 64],
            pv_table: [[Default::default(); 64]; 64],
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

pub fn search(board: &mut Board, terminator: Arc<RwLock<bool>>, depth: u32) {
    let mut thread = SearchThread::new(terminator);
    let mut last_best = Default::default();

    for current in 1..=depth {
        thread.nodes = 0;

        let now = Instant::now();

        let p = SearchParams::new(board, Score::NEGATIVE_INFINITY, Score::INFINITY, current, 0);
        let score = negamax::negamax_search(p, &mut thread);

        let duration = now.elapsed();

        let mut pv = vec![];
        let mut index = 0;
        while thread.pv_table[0][index] != Default::default() {
            pv.push(thread.pv_table[0][index]);
            index += 1;
        }

        if *thread.terminator.read().unwrap() {
            uci::send(UciMessage::BestMove(last_best));
            return;
        }

        last_best = pv[0];

        uci::send(UciMessage::SearchReport {
            depth: current,
            score,
            duration,
            pv: pv.to_vec(),
            nodes: thread.nodes,
        });

        if current == depth {
            uci::send(UciMessage::BestMove(last_best));
        }
    }
}
