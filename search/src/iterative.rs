use game::{Board, Move, Score};

use super::{negamax, SearchParams, SearchThread};

const WINDOW_MARGIN: Score = Score(50);

pub fn iterative_search(mut board: Board, mut thread: SearchThread) {
    let mut last_best = Default::default();

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

        report_search_result(depth, score, &pv, &thread);

        last_best = pv[0];
        depth += 1;

        // thread.nodes = 0;
        // thread.start_time = Instant::now();
    }

    println!("bestmove {}", last_best);
}

fn interrupted(thread: &SearchThread) -> bool {
    thread.is_time_over() || thread.requested_termination()
}

fn report_search_result(depth: usize, score: Score, pv: &[Move], thread: &SearchThread) {
    let duration = thread.start_time.elapsed();

    let nps = thread.nodes as f32 / duration.as_secs_f32();
    let ms = duration.as_millis();

    print!(
        "info depth {} score cp {} nodes {} time {} nps {:.0} pv",
        depth, score, thread.nodes, ms, nps
    );
    pv.iter().for_each(|mv| print!(" {}", mv));
    println!();
}
