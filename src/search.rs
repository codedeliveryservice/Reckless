use std::time::Instant;

use crate::evaluation::{INFINITY, checkmate_in};
use crate::types::Move;
use crate::{board::Board, search::alphabeta::AlphaBetaSearch};

pub use self::thread::SearchThread;

mod alphabeta;
mod ordering;
mod quiescence;
mod thread;

const ASPIRATION_WINDOW_MARGIN: i32 = 50;

/// Iterative deepening is a search algorithm that incrementally explores deeper levels of
/// the search space by iteratively calling a depth-limited version of the depth-first search.
///
/// It continues to increase the depth with each iteration until the time limit is reached
/// or the search is terminated by the user.
///
/// Despite intuitively appearing inefficient, iterative deepening is essential in implementing
/// time management. By utilizing dynamic move ordering techniques, it effectively leads to
/// numerous alpha-beta cutoffs, resulting in a significant reduction in the number of nodes
/// that need to be searched again.
pub struct IterativeSearch {
    board: Board,
    thread: SearchThread,
    alpha: i32,
    beta: i32,
}

impl IterativeSearch {
    /// Creates a new `IterativeSearch` instance.
    pub fn new(board: Board, thread: SearchThread) -> Self {
        Self {
            board,
            thread,
            alpha: -INFINITY,
            beta: INFINITY,
        }
    }

    /// Performs an iterative deepening search until the time limit is reached or the search is terminated.
    pub fn search(&mut self) {
        let mut last_best = Move::default();
        let mut depth = 1;

        while depth <= self.thread.tc.get_max_depth() {
            self.board.ply = 0;
            self.thread.nodes = 0;
            self.thread.current_depth = depth;

            let stopwatch = Instant::now();
            let score = AlphaBetaSearch::new(&mut self.board, &mut self.thread).search(self.alpha, self.beta, depth);

            if self.thread.get_terminator() {
                break;
            }

            if !self.is_score_within_bounds(score) {
                self.reset_aspiration_window();
                continue;
            }

            self.update_aspiration_window(score);

            let pv = self.thread.get_principal_variation(&mut self.board, depth);
            self.report_search_result(depth, score, &pv, stopwatch);

            last_best = pv[0];
            depth += 1;
        }

        println!("bestmove {last_best}");
    }

    /// Returns `true` if the given score is within the aspiration window.
    fn is_score_within_bounds(&self, score: i32) -> bool {
        self.alpha < score && score < self.beta
    }

    /// Updates the aspiration window to be centered around the given score.
    fn update_aspiration_window(&mut self, score: i32) {
        self.alpha = score - ASPIRATION_WINDOW_MARGIN;
        self.beta = score + ASPIRATION_WINDOW_MARGIN;
    }

    /// Resets the aspiration window to its default values (unbounded).
    fn reset_aspiration_window(&mut self) {
        self.alpha = -INFINITY;
        self.beta = INFINITY;
    }

    /// Reports the result of a search iteration using the `info` UCI command.
    fn report_search_result(&self, depth: usize, score: i32, pv: &[Move], stopwatch: Instant) {
        let nodes = self.thread.nodes;
        let nps = nodes as f32 / stopwatch.elapsed().as_secs_f32();
        let ms = stopwatch.elapsed().as_millis();

        let hashfull = self.thread.cache.lock().unwrap().get_load_factor();
        let score = match checkmate_in(score) {
            Some(moves) => format!("mate {moves}"),
            None => format!("cp {score}"),
        };

        print!("info depth {depth} score {score} nodes {nodes} time {ms} nps {nps:.0} hashfull {hashfull} pv");
        for mv in pv {
            print!(" {mv}");
        }
        println!();
    }
}
