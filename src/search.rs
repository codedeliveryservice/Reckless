use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};

use crate::{
    board::Board,
    search::thread::SearchThread,
    tables::{CorrectionHistory, History, TranspositionTable},
    time::Limits,
    types::{Move, Score},
};

mod alphabeta;
mod aspiration;
mod counter;
mod ordering;
mod quiescence;
mod thread;

static NODES_GLOBAL: AtomicU64 = AtomicU64::new(0);
static ABORT_SIGNAL: AtomicBool = AtomicBool::new(false);

pub struct Options {
    pub silent: bool,
    pub threads: usize,
    pub limits: Limits,
}

// Used by the 'datagen' feature; allow dead code warnings.
#[allow(dead_code)]
pub struct SearchResult {
    pub best_move: Move,
    pub score: i32,
    pub nodes: u64,
}

pub fn start(
    options: Options,
    board: &mut Board,
    history: &mut History,
    corrhist: &mut CorrectionHistory,
    tt: &TranspositionTable,
) -> SearchResult {
    NODES_GLOBAL.store(0, Ordering::Relaxed);
    ABORT_SIGNAL.store(false, Ordering::Relaxed);

    std::thread::scope(|scope| {
        let mut threads = Vec::new();

        for _ in 0..(options.threads - 1) {
            let mut board = board.clone();
            let mut history = history.clone();
            let mut corrhist = corrhist.clone();

            let thread = scope.spawn(move || {
                let thread = SearchThread::new(Limits::Infinite, &mut board, &mut history, &mut corrhist, tt);
                iterative_deepening(thread, true)
            });

            threads.push(thread);
        }

        let thread = SearchThread::new(options.limits, board, history, corrhist, tt);
        let result = iterative_deepening(thread, options.silent);

        ABORT_SIGNAL.store(true, Ordering::Relaxed);
        for thread in threads {
            thread.join().unwrap();
        }
        result
    })
}

fn iterative_deepening(mut thread: SearchThread, silent: bool) -> SearchResult {
    let now = Instant::now();

    let mut current_score = 0;

    for depth in 1.. {
        let score = thread.aspiration_search(current_score, depth);

        if thread.stopped {
            break;
        }

        if !silent {
            print_uci_info(&thread, depth, score, now);
        }

        current_score = score;

        thread.sel_depth = 0;
        thread.finished_depth = depth;
        thread.time_manager.update(depth, score, thread.pv_table.best_move());

        let effort = thread.node_table.get(thread.pv_table.best_move()) as f64 / thread.nodes.local() as f64;
        if thread.time_manager.if_finished(depth, effort) {
            break;
        }
    }

    if !silent {
        println!("bestmove {}", thread.pv_table.best_move());
    }

    SearchResult {
        best_move: thread.pv_table.best_move(),
        score: current_score,
        nodes: thread.nodes.global(),
    }
}

fn print_uci_info(thread: &SearchThread, depth: i32, score: i32, now: Instant) {
    let nps = thread.nodes.global() as f64 / now.elapsed().as_secs_f64();
    let ms = now.elapsed().as_millis();

    let score = match score {
        s if s > Score::MATE_BOUND => format!("mate {}", (Score::MATE - score + 1) / 2),
        s if s < -Score::MATE_BOUND => format!("mate {}", (-Score::MATE - score) / 2),
        _ => format!("cp {score}"),
    };

    print!(
        "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {} pv",
        thread.sel_depth,
        thread.nodes.global(),
        thread.tt.hashfull(),
    );
    for mv in &thread.pv_table.variation() {
        print!(" {mv}");
    }
    println!();
}
