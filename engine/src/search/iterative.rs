use std::time::Instant;

use game::{Board, Move, Score};

use super::{negamax, SearchParams, SearchThread};
use crate::uci::{self, UciMessage};

const WINDOW_MARGIN: Score = Score(50);

pub fn iterative_search(mut board: Board, mut thread: SearchThread) {
    let mut last_best = Default::default();

    let mut alpha = Score::NEGATIVE_INFINITY;
    let mut beta = Score::INFINITY;
    let mut depth = 1;

    while depth <= thread.tc.max_depth {
        let params = SearchParams::new(&mut board, alpha, beta, depth, 0);
        let score = negamax::negamax_search(params, &mut thread);

        if interrupted(&thread) {
            break;
        }

        // Reset the window on failure and try again at the same depth with a full width window
        if score <= alpha || score >= beta {
            alpha = Score::NEGATIVE_INFINITY;
            beta = Score::INFINITY;
            continue;
        }

        // Set the window for the next iteration
        alpha = score - WINDOW_MARGIN;
        beta = score + WINDOW_MARGIN;

        let mut pv = vec![];
        thread.extract_pv_line(&mut board, depth, &mut pv);

        report_search_result(depth, score, &pv, &thread);

        last_best = pv[0];
        depth += 1;

        thread.nodes = 0;
        thread.start_time = Instant::now();
    }

    uci::send(UciMessage::BestMove(last_best));
}

fn interrupted(thread: &SearchThread) -> bool {
    thread.tc.is_time_over() || thread.requested_termination()
}

fn report_search_result(depth: usize, score: Score, pv: &Vec<Move>, thread: &SearchThread) {
    uci::send(UciMessage::SearchReport {
        pv,
        depth,
        score,
        nodes: thread.nodes,
        duration: thread.start_time.elapsed(),
    });
}
