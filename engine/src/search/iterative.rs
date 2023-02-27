use std::time::Instant;

use game::{Board, Score};

use super::{negamax, SearchParams, SearchThread};
use crate::uci::{self, UciMessage};

pub fn iterative_search(board: &mut Board, mut thread: SearchThread) {
    let mut last_best = Default::default();

    for depth in 1..=thread.tc.max_depth {
        thread.nodes = 0;

        let now = Instant::now();
        let params = SearchParams::new(board, Score::NEGATIVE_INFINITY, Score::INFINITY, depth, 0);
        let score = negamax::negamax_search(params, &mut thread);
        let duration = now.elapsed();

        if thread.tc.is_time_over() || thread.requested_termination() {
            uci::send(UciMessage::BestMove(last_best));
            return;
        }

        let mut pv = vec![];
        thread.extract_pv_line(board, depth, &mut pv);
        last_best = pv[0];

        uci::send(UciMessage::SearchReport {
            depth,
            score,
            duration,
            pv: &pv,
            nodes: thread.nodes,
        });
    }

    uci::send(UciMessage::BestMove(last_best));
}
