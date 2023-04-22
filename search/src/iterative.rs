use std::time::Instant;

use game::{Board, Move, Score};

use super::{negamax, SearchParams, SearchThread};

const WINDOW_MARGIN: Score = Score(50);

pub fn iterative_search(mut board: Board, mut thread: SearchThread) {
    let mut last_best = Default::default();
    let mut stopwatch = Instant::now();
    let mut nodes = 0;

    let mut alpha = -Score::INFINITY;
    let mut beta = Score::INFINITY;
    let mut depth = 1;

    while depth <= thread.tc.get_max_depth() {
        let params = SearchParams::new(&mut board, alpha, beta, depth, 0);
        let score = negamax::negamax_search(params, &mut thread);

        if interrupted(&thread) {
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

        let mut pv = vec![];
        thread.extract_pv_line(&mut board, depth, &mut pv);

        report_search_result(depth, score, &pv, &stopwatch, thread.nodes - nodes);

        last_best = pv[0];
        depth += 1;

        stopwatch = Instant::now();
        nodes = thread.nodes;
    }

    println!("bestmove {}", last_best);
}

fn interrupted(thread: &SearchThread) -> bool {
    thread.is_time_over() || thread.get_terminator()
}

fn report_search_result(depth: usize, score: Score, pv: &[Move], stopwatch: &Instant, nodes: u32) {
    let duration = stopwatch.elapsed();

    let nps = nodes as f32 / duration.as_secs_f32();
    let ms = duration.as_millis();

    print!(
        "info depth {} score cp {} nodes {} time {} nps {:.0} pv",
        depth, score, nodes, ms, nps
    );
    pv.iter().for_each(|mv| print!(" {}", mv));
    println!();
}
