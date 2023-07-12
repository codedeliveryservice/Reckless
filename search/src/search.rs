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

const WINDOW_MARGIN: Score = Score(50);

pub struct IterativeSearch {
    board: Board,
    thread: SearchThread,
}

impl IterativeSearch {
    pub fn new(board: Board, thread: SearchThread) -> Self {
        Self { board, thread }
    }

    pub fn search(&mut self) {
        let mut last_best = Default::default();

        let mut alpha = -Score::INFINITY;
        let mut beta = Score::INFINITY;
        let mut depth = 1;

        while depth <= self.thread.tc.get_max_depth() {
            let stopwatch = Instant::now();
            let mut search = AlphaBetaSearch::new(&mut self.board, &mut self.thread);
            let score = search.search(SearchParams::new(alpha, beta, depth));

            if self.thread.get_terminator() {
                break;
            }

            // Reset the window on failure and try again at the same depth with a full width window
            if score <= alpha || score >= beta {
                alpha = -Score::INFINITY;
                beta = Score::INFINITY;
                continue;
            }

            // Set the window for the next iteration
            alpha = score - WINDOW_MARGIN;
            beta = score + WINDOW_MARGIN;

            let pv = self.thread.get_principal_variation(&mut self.board, depth);

            self.report_search_result(depth, score, &pv, stopwatch);

            last_best = pv[0];
            depth += 1;
            self.thread.nodes = 0;
            self.thread.current_depth = depth;
        }

        println!("bestmove {}", last_best);
    }

    fn report_search_result(&self, depth: usize, score: Score, pv: &[Move], stopwatch: Instant) {
        let duration = stopwatch.elapsed();
        let nps = self.thread.nodes as f32 / duration.as_secs_f32();
        let ms = duration.as_millis();

        let score = match score.checkmate_in() {
            Some(moves) => format!("mate {}", moves),
            None => format!("cp {}", score),
        };

        print!(
            "info depth {} score {} nodes {} time {} nps {:.0} pv",
            depth, score, self.thread.nodes, ms, nps
        );
        pv.iter().for_each(|mv| print!(" {}", mv));
        println!();
    }
}
