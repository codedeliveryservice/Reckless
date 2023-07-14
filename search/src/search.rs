use std::time::Instant;

use game::{Board, Move, Score};

pub use alphabeta::*;
pub use ordering::*;
pub use params::*;
pub use quiescence::*;
pub use thread::*;

mod alphabeta;
mod ordering;
mod params;
mod quiescence;
mod thread;

const ASPIRATION_WINDOW_MARGIN: Score = Score(50);

pub struct IterativeSearch {
    board: Board,
    thread: SearchThread,
    alpha: Score,
    beta: Score,
}

impl IterativeSearch {
    /// Creates a new `IterativeSearch` instance.
    pub fn new(board: Board, thread: SearchThread) -> Self {
        Self {
            board,
            thread,
            alpha: -Score::INFINITY,
            beta: Score::INFINITY,
        }
    }

    /// Performs an iterative deepening search to find the best move.
    pub fn search(&mut self) {
        let mut last_best = Default::default();
        let mut depth = 1;

        while depth <= self.thread.tc.get_max_depth() {
            let stopwatch = Instant::now();
            let score = self.alphabeta_search(depth);

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

            self.thread.nodes = 0;
            self.thread.current_depth = depth;
        }

        println!("bestmove {}", last_best);
    }

    /// Performs a search at the given depth.
    fn alphabeta_search(&mut self, depth: usize) -> Score {
        let mut search = AlphaBetaSearch::new(&mut self.board, &mut self.thread);
        search.search(SearchParams::new(self.alpha, self.beta, depth))
    }

    /// Returns `true` if the given score is within the aspiration window.
    fn is_score_within_bounds(&self, score: Score) -> bool {
        self.alpha < score && score < self.beta
    }

    /// Updates the aspiration window to be centered around the given score.
    fn update_aspiration_window(&mut self, score: Score) {
        self.alpha = score - ASPIRATION_WINDOW_MARGIN;
        self.beta = score + ASPIRATION_WINDOW_MARGIN;
    }

    /// Resets the aspiration window to its default values (unbounded).
    fn reset_aspiration_window(&mut self) {
        self.alpha = -Score::INFINITY;
        self.beta = Score::INFINITY;
    }

    /// Reports the result of a search iteration using the `info` UCI command.
    fn report_search_result(&self, depth: usize, score: Score, pv: &[Move], stopwatch: Instant) {
        let nodes = self.thread.nodes;
        let nps = nodes as f32 / stopwatch.elapsed().as_secs_f32();
        let ms = stopwatch.elapsed().as_millis();

        let score = match score.checkmate_in() {
            Some(moves) => format!("mate {}", moves),
            None => format!("cp {}", score),
        };

        print!(
            "info depth {} score {} nodes {} time {} nps {:.0} pv",
            depth, score, nodes, ms, nps
        );
        pv.iter().for_each(|mv| print!(" {}", mv));
        println!();
    }
}
