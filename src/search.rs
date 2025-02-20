use std::time::Instant;

use crate::{
    movepick::MovePicker,
    thread::ThreadData,
    types::{mated_in, Move, Score, MAX_PLY},
};

const PV: bool = true;
const NON_PV: bool = false;

pub fn start(td: &mut ThreadData, silent: bool) {
    td.nodes = 0;
    td.completed_depth = 0;
    td.stopped = false;
    td.pv.clear(0);

    let now = Instant::now();

    for depth in 1..MAX_PLY as i32 {
        let score = search::<PV>(td, -Score::INFINITE, Score::INFINITE, depth);

        if td.stopped {
            break;
        }

        if !silent {
            td.print_uci_info(depth, score, now);
        }

        td.completed_depth = depth;

        if td.time_manager.soft_limit(depth, td.nodes) {
            break;
        }
    }
}

fn search<const PV: bool>(td: &mut ThreadData, mut alpha: i32, beta: i32, depth: i32) -> i32 {
    let is_root = td.ply == 0;
    let in_check = td.board.in_check();

    td.pv.clear(td.ply);

    if depth <= 0 {
        return qsearch(td, alpha, beta);
    }

    td.nodes += 1;

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if !is_root {
        if td.board.is_draw() {
            return Score::DRAW;
        }

        if td.ply >= MAX_PLY - 1 {
            return if in_check { Score::DRAW } else { td.board.evaluate() };
        }
    }

    let mut best_score = -Score::INFINITE;

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(td);

    while let Some(mv) = move_picker.next() {
        if !td.board.make_move::<true, false>(mv) {
            td.board.undo_move::<true>();
            continue;
        }

        move_count += 1;
        td.ply += 1;

        let score = -search::<PV>(td, -beta, -alpha, depth - 1);

        td.board.undo_move::<true>();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                if PV {
                    td.pv.update(td.ply, mv);
                }

                if score >= beta {
                    break;
                }

                alpha = score;
            }
        }
    }

    if move_count == 0 {
        return if in_check { mated_in(td.ply) } else { Score::DRAW };
    }

    best_score
}

fn qsearch(td: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    let in_check = td.board.in_check();

    td.nodes += 1;

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.ply >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { td.board.evaluate() };
    }

    let mut best_score = td.board.evaluate();

    if best_score >= beta {
        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let mut move_picker = MovePicker::new_noisy(td);

    while let Some(mv) = move_picker.next() {
        if !td.board.make_move::<true, false>(mv) {
            td.board.undo_move::<true>();
            continue;
        }

        td.ply += 1;

        let score = -qsearch(td, -beta, -alpha);

        td.board.undo_move::<true>();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                if score >= beta {
                    break;
                }

                alpha = score;
            }
        }
    }

    best_score
}
