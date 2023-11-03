use std::time::Instant;

use crate::board::Board;
use crate::cache::{Bound, Cache};
use crate::tables::{HistoryMoves, KillerMoves};
use crate::timeman::{Limits, TimeManager};
use crate::types::{Move, Score};

mod alphabeta;
mod aspiration;
mod ordering;
mod quiescence;
mod selectivity;

pub struct Searcher<'a> {
    pub nodes: u32,
    pub stopped: bool,
    pub print_to_stdout: bool,
    board: Board,
    cache: &'a mut Cache,
    time_manager: TimeManager,
    killers: KillerMoves,
    history: HistoryMoves,
    sel_depth: usize,
}

impl<'a> Searcher<'a> {
    /// Creates a new `Searcher` instance.
    pub fn new(board: Board, limits: Limits, cache: &'a mut Cache) -> Self {
        Self {
            board,
            cache,
            time_manager: TimeManager::new(limits),
            killers: KillerMoves::default(),
            history: HistoryMoves::default(),
            sel_depth: Default::default(),
            nodes: Default::default(),
            stopped: Default::default(),
            print_to_stdout: true,
        }
    }

    /// Incrementally explores deeper levels of the game tree using iterative deepening.
    ///
    /// The iterative deepening algorithm is a strategy that involves doing a series of depth-limited
    /// depth-first searches on the game tree, starting with a shallow search and gradually increases
    /// the depth until the time limit is reached or the search is terminated.
    pub fn iterative_deepening(&mut self) {
        let stopwatch = Instant::now();

        let mut last_best = Move::default();
        let mut last_score = Default::default();

        for depth in 1..=self.time_manager.get_max_depth() {
            let score = match depth {
                1..=6 => self.alpha_beta::<true, true>(-Score::INFINITY, Score::INFINITY, depth),
                _ => self.aspiration_window(last_score, depth),
            };

            if self.stopped {
                break;
            }

            if self.print_to_stdout {
                self.report_search_result(depth, score, stopwatch);
            }

            last_score = score;
            last_best = self.cache.read(self.board.hash(), 0).unwrap().mv;
            self.sel_depth = 0;

            if self.time_manager.is_soft_bound_reached() {
                break;
            }
        }

        if self.print_to_stdout {
            println!("bestmove {last_best}");
        }
    }

    /// Reports the result of a search iteration using the `info` UCI command.
    fn report_search_result(&mut self, depth: i32, score: i32, stopwatch: Instant) {
        let nps = self.nodes as f32 / stopwatch.elapsed().as_secs_f32();
        let ms = stopwatch.elapsed().as_millis();

        let hashfull = self.cache.get_load_factor();
        let score = format_score(score);

        print!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {hashfull} pv",
            self.sel_depth, self.nodes,
        );
        self.print_principle_variation();
        println!();
    }

    /// Recursively prints the principle variation.
    fn print_principle_variation(&mut self) {
        if let Some(entry) = self.cache.read(self.board.hash(), 0) {
            if entry.bound == Bound::Exact && !self.board.is_repetition() {
                print!(" {}", entry.mv);
                self.board.make_move(entry.mv).unwrap();
                self.print_principle_variation();
                self.board.undo_move();
            }
        }
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
    format!("cp {score}")
}
