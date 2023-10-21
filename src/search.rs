use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::tables::{HistoryMoves, KillerMoves};
use crate::timeman::{Limits, TimeManager};
use crate::types::{Move, Score};
use crate::{board::Board, cache::Cache};

mod alphabeta;
mod ordering;
mod quiescence;

const ASPIRATION_WINDOW_MARGIN: i32 = 50;

pub struct Searcher {
    pub nodes: u32,
    pub print_to_stdout: bool,
    board: Board,
    cache: Arc<Mutex<Cache>>,
    terminator: Arc<AtomicBool>,
    time_manager: TimeManager,
    killers: KillerMoves,
    history: HistoryMoves,
    sel_depth: usize,
}

impl Searcher {
    /// Creates a new `Searcher` instance.
    pub fn new(board: Board, limits: Limits, terminator: Arc<AtomicBool>, cache: Arc<Mutex<Cache>>) -> Self {
        Self {
            board,
            cache,
            terminator,
            time_manager: TimeManager::new(limits),
            killers: KillerMoves::default(),
            history: HistoryMoves::default(),
            sel_depth: 0,
            nodes: 0,
            print_to_stdout: true,
        }
    }

    /// Incrementally explores deeper levels of the game tree using iterative deepening.
    ///
    /// The iterative deepening algorithm is a strategy that involves doing a series of depth-limited
    /// depth-first searches on the game tree, starting with a shallow search and gradually increases
    /// the depth until the time limit is reached or the search is terminated.
    pub fn iterative_deepening(&mut self) {
        self.board.ply = 0;

        let stopwatch = Instant::now();

        let mut last_best = Move::default();
        let mut depth = 1;

        let mut alpha = -Score::INFINITY;
        let mut beta = Score::INFINITY;

        while depth <= self.time_manager.get_max_depth() {
            let score = self.alpha_beta(alpha, beta, depth);

            if self.load_terminator() {
                break;
            }

            if score <= alpha || score >= beta {
                alpha = -Score::INFINITY;
                beta = Score::INFINITY;
                continue;
            }

            alpha = score - ASPIRATION_WINDOW_MARGIN;
            beta = score + ASPIRATION_WINDOW_MARGIN;

            if self.print_to_stdout {
                self.report_search_result(depth, score, stopwatch);
            }

            last_best = self.get_best_move(&self.board).unwrap();
            depth += 1;
            self.sel_depth = 0;
        }

        if self.print_to_stdout {
            println!("bestmove {last_best}");
        }
    }

    /// Reports the result of a search iteration using the `info` UCI command.
    fn report_search_result(&mut self, depth: usize, score: i32, stopwatch: Instant) {
        let nps = self.nodes as f32 / stopwatch.elapsed().as_secs_f32();
        let ms = stopwatch.elapsed().as_millis();

        let hashfull = self.cache.lock().unwrap().get_load_factor();
        let score = format_score(score);

        let pv = self.get_principal_variation(depth);
        let pv = pv.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(" ");

        println!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {hashfull} pv {pv}",
            self.sel_depth, self.nodes,
        );
    }

    /// Returns `true` if the search has been terminated.
    fn load_terminator(&self) -> bool {
        self.terminator.load(Ordering::Relaxed)
    }

    /// Stores the search termination flag.
    fn store_terminator(&self, value: bool) {
        self.terminator.store(value, Ordering::Relaxed)
    }

    /// Extracts the best move from the transposition table.
    fn get_best_move(&self, board: &Board) -> Option<Move> {
        self.cache.lock().unwrap().read(board.hash(), 0).map(|entry| entry.mv)
    }

    /// Extracts the principal variation line from the transposition table limited to the given depth.
    fn get_principal_variation(&mut self, mut depth: usize) -> Vec<Move> {
        let mut pv_line = Vec::with_capacity(depth);

        let cache = self.cache.lock().unwrap();
        while depth != 0 {
            if let Some(entry) = cache.read(self.board.hash(), 0) {
                pv_line.push(entry.mv);
                self.board.make_move(entry.mv).unwrap();
                depth -= 1;
            } else {
                break;
            }
        }

        pv_line.iter().for_each(|_| self.board.undo_move());
        pv_line
    }
}

/// Formats a score in UCI format.
fn format_score(score: i32) -> String {
    if score > Score::CHECKMATE_BOUND {
        return format!("mate {}", (Score::CHECKMATE - score + 1) / 2);
    }
    if score < -Score::CHECKMATE_BOUND {
        return format!("mate {}", (-Score::CHECKMATE - score) / 2);
    }
    format!("cp {}", score)
}
