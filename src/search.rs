use std::time::Instant;

use crate::{
    movepick::MovePicker,
    thread::ThreadData,
    transposition::Bound,
    types::{is_decisive, mated_in, ArrayVec, Move, Score, MAX_PLY},
};

pub fn start(td: &mut ThreadData, silent: bool) {
    td.nodes = 0;
    td.completed_depth = 0;
    td.stopped = false;
    td.pv.clear(0);

    let now = Instant::now();

    for depth in 1..MAX_PLY as i32 {
        let score = search::<true>(td, -Score::INFINITE, Score::INFINITE, depth);

        if !silent {
            td.print_uci_info(depth, score, now);
        }

        if td.stopped {
            break;
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

    let entry = td.tt.read(td.board.hash(), td.ply);
    let tt_move = entry.map(|entry| entry.mv).unwrap_or(Move::NULL);

    if let Some(entry) = entry {
        if !PV
            && entry.depth >= depth
            && match entry.bound {
                Bound::Upper => entry.score <= alpha,
                Bound::Lower => entry.score >= beta,
                _ => true,
            }
        {
            return entry.score;
        }
    }

    let eval = if in_check { Score::NONE } else { td.board.evaluate() };

    if !PV && !in_check && depth <= 8 && eval - 80 * depth >= beta {
        return eval;
    }

    if !PV
        && !in_check
        && depth >= 3
        && eval >= beta
        && td.stack[td.ply - 1].mv != Move::NULL
        && td.board.has_non_pawns()
    {
        let r = 3 + depth / 3 + ((eval - beta) / 256).min(3);

        td.stack[td.ply].mv = Move::NULL;
        td.board.make_null_move();
        let score = -search::<false>(td, -beta, -beta + 1, depth - r);
        td.board.undo_null_move();

        match score {
            s if is_decisive(s) => return beta,
            s if s >= beta => return s,
            _ => (),
        }
    }

    let mut best_score = -Score::INFINITE;
    let mut best_move = Move::NULL;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(td, tt_move);

    while let Some(mv) = move_picker.next() {
        if !td.board.make_move::<true, false>(mv) {
            td.board.undo_move::<true>(mv);
            continue;
        }

        td.stack[td.ply].mv = mv;
        td.ply += 1;
        move_count += 1;

        let mut score = Score::ZERO;

        if !PV || move_count > 1 {
            score = -search::<false>(td, -alpha - 1, -alpha, depth - 1);
        }

        if PV && (move_count == 1 || score > alpha) {
            score = -search::<true>(td, -beta, -alpha, depth - 1);
        }

        td.board.undo_move::<true>(mv);
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = mv;

                if PV {
                    td.pv.update(td.ply, mv);
                }

                if score >= beta {
                    update_histories(td, best_move, quiet_moves, depth);
                    break;
                }

                alpha = score;
            }
        }

        if mv != best_move && move_count < 32 && !mv.is_noisy() {
            quiet_moves.push(mv);
        }
    }

    if move_count == 0 {
        return if in_check { mated_in(td.ply) } else { Score::DRAW };
    }

    let bound = if best_score >= beta {
        Bound::Lower
    } else if best_move == Move::NULL {
        Bound::Upper
    } else {
        Bound::Exact
    };

    td.tt.write(td.board.hash(), depth, best_score, bound, best_move, td.ply);

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
            td.board.undo_move::<true>(mv);
            continue;
        }

        td.stack[td.ply].mv = mv;
        td.ply += 1;

        let score = -qsearch(td, -beta, -alpha);

        td.board.undo_move::<true>(mv);
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

fn update_histories(td: &mut ThreadData, best_move: Move, quiet_moves: ArrayVec<Move, 32>, depth: i32) {
    if best_move.is_noisy() {
        return;
    }

    td.main_history.update(&td.board, best_move, quiet_moves, depth);
}
