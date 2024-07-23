use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
    board::Board,
    tables::{History, TranspositionTable},
    time::Limits,
    types::Move,
};

use self::thread::SearchThread;

mod alphabeta;
mod aspiration;
mod counter;
mod deepening;
mod ordering;
mod quiescence;
mod see;
mod thread;
mod parameters;

static NODES_GLOBAL: AtomicU64 = AtomicU64::new(0);
static ABORT_SIGNAL: AtomicBool = AtomicBool::new(false);

pub struct Options {
    pub silent: bool,
    pub threads: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: i32,
    pub nodes: u64,
}

pub fn start(options: Options, limits: Limits, board: &mut Board, history: &mut History, tt: &TranspositionTable) -> SearchResult {
    NODES_GLOBAL.store(0, Ordering::Relaxed);
    ABORT_SIGNAL.store(false, Ordering::Relaxed);

    std::thread::scope(|scope| {
        let mut threads = Vec::new();

        for _ in 0..(options.threads - 1) {
            let mut board = board.clone();
            let mut history = history.clone();

            let thread = scope.spawn(move || {
                let mut searcher = SearchThread::new(Limits::Infinite, &mut board, &mut history, tt);
                searcher.silent = true;
                searcher.run()
            });

            threads.push(thread);
        }

        let mut searcher = SearchThread::new(limits, board, history, tt);
        searcher.silent = options.silent;

        let result = searcher.run();

        ABORT_SIGNAL.store(true, Ordering::Relaxed);
        for thread in threads {
            thread.join().unwrap();
        }

        result
    })
}
