use std::time::Instant;

use super::SearchResult;
use crate::types::{Move, Score};

const ASPIRATION_WINDOW_THRESHOLD: i32 = 6;
const NODE_TM_DEPTH_THRESHOLD: i32 = 10;

impl super::Searcher<'_> {
    /// Incrementally explores deeper levels of the game tree using iterative deepening.
    ///
    /// The iterative deepening algorithm is a strategy that involves doing a series of depth-limited
    /// depth-first searches on the game tree, starting with a shallow search and gradually increases
    /// the depth until the time limit is reached or the search is terminated.
    pub fn iterative_deepening(&mut self) -> SearchResult {
        let stopwatch = Instant::now();

        let mut result = SearchResult {
            best_move: Move::NULL,
            score: Score::INVALID,
        };

        for depth in 1..=self.time_manager.max_depth() {
            let score = if depth > ASPIRATION_WINDOW_THRESHOLD {
                self.aspiration_search(result.score, depth)
            } else {
                self.alpha_beta::<true, true>(-Score::INFINITY, Score::INFINITY, depth)
            };

            if self.stopped {
                break;
            }

            if !self.silent {
                self.report_search_result(depth, score, stopwatch);
            }

            result.best_move = self.cache.read(self.board.hash(), 0).unwrap().mv;
            result.score = score;

            self.sel_depth = 0;
            self.finished_depth = depth;

            if self.finished_depth >= NODE_TM_DEPTH_THRESHOLD {
                let effort = self.node_table.get(result.best_move) as f64 / self.nodes as f64;
                self.time_manager.adjust(effort);
            }

            if self.time_manager.is_soft_bound_reached() {
                break;
            }
        }

        if !self.silent {
            println!("bestmove {}", result.best_move);
        }
        result
    }

    /// Reports the result of a search iteration using the `info` UCI command.
    fn report_search_result(&mut self, depth: i32, score: i32, stopwatch: Instant) {
        let nps = self.nodes as f32 / stopwatch.elapsed().as_secs_f32();
        let ms = stopwatch.elapsed().as_millis();

        let hashfull = self.cache.get_load_factor();
        let score = self.format_score(score);

        print!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {hashfull} pv",
            self.sel_depth, self.nodes,
        );
        self.pv_table.get_line().iter().for_each(|mv| print!(" {mv}"));
        println!();
    }

    /// Formats a score in UCI format.
    fn format_score(&self, score: i32) -> String {
        if score > Score::MATE_BOUND {
            return format!("mate {}", (Score::MATE - score + 1) / 2);
        }
        if score < -Score::MATE_BOUND {
            return format!("mate {}", (-Score::MATE - score) / 2);
        }
        format!("cp {score}")
    }
}
