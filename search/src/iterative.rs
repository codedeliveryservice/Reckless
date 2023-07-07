use std::time::Instant;

use game::{Board, Move, Score};

use super::{alphabeta::AlphaBetaSearch, SearchParams, SearchThread};

const WINDOW_MARGIN: Score = Score(50);

pub fn iterative_search(mut board: Board, mut thread: SearchThread) {
    let mut last_best = Default::default();

    let mut alpha = -Score::INFINITY;
    let mut beta = Score::INFINITY;
    let mut depth = 1;

    while depth <= thread.tc.get_max_depth() {
        let mut search = AlphaBetaSearch::new(&mut board, &mut thread);
        let score = search.search(SearchParams::new(alpha, beta, depth));
        let stopwatch = search.start_time;

        if thread.get_terminator() {
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

        let pv = thread.get_principal_variation(&mut board, depth);

        report_search_result(depth, score, &pv, stopwatch, thread.nodes);

        last_best = pv[0];
        thread.nodes = 0;
        depth += 1;
        thread.current_depth = depth;
    }

    println!("bestmove {}", last_best);
}

fn report_search_result(depth: usize, score: Score, pv: &[Move], stopwatch: Instant, nodes: u32) {
    let duration = stopwatch.elapsed();
    let nps = nodes as f32 / duration.as_secs_f32();
    let ms = duration.as_millis();

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
