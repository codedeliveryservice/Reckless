use std::time::Instant;

use crate::{
    movepick::MovePicker,
    parameters::lmp_threshold,
    thread::ThreadData,
    transposition::Bound,
    types::{is_decisive, is_loss, mated_in, ArrayVec, Move, Score, MAX_PLY},
};

pub fn start(td: &mut ThreadData, silent: bool) {
    td.nodes = 0;
    td.completed_depth = 0;
    td.stopped = false;
    td.pv.clear(0);

    let now = Instant::now();

    let mut score = Score::NONE;
    let mut alpha = -Score::INFINITE;
    let mut beta = Score::INFINITE;
    let mut delta = 24;

    for depth in 1..MAX_PLY as i32 {
        if depth >= 4 {
            alpha = (score - delta).max(-Score::INFINITE);
            beta = (score + delta).min(Score::INFINITE);
        }

        loop {
            score = search::<true>(td, alpha, beta, depth, false);

            if td.stopped {
                break;
            }

            match score {
                s if s <= alpha => {
                    beta = (alpha + beta) / 2;
                    alpha = (score - delta).max(-Score::INFINITE);
                }
                s if s >= beta => {
                    beta = (score + delta).min(Score::INFINITE);
                }
                _ => break,
            }

            delta += delta / 2;
        }

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

fn search<const PV: bool>(td: &mut ThreadData, mut alpha: i32, beta: i32, depth: i32, cut_node: bool) -> i32 {
    let is_root = td.ply == 0;
    let in_check = td.board.in_check();

    td.pv.clear(td.ply);

    if depth <= 0 && !in_check {
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

    let depth = depth.max(1);

    let entry = td.tt.read(td.board.hash(), td.ply);
    let mut tt_move = Move::NULL;
    let mut tt_pv = PV;

    if let Some(entry) = entry {
        tt_move = entry.mv;
        tt_pv |= entry.pv;

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
        td.ply += 1;

        td.board.make_null_move();

        let score = -search::<false>(td, -beta, -beta + 1, depth - r, !cut_node);

        td.board.undo_null_move();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        match score {
            s if is_decisive(s) => return beta,
            s if s >= beta => return s,
            _ => (),
        }
    }

    let mut best_score = -Score::INFINITE;
    let mut best_move = Move::NULL;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(td, tt_move);
    let mut skip_quiets = false;

    while let Some((mv, _)) = move_picker.next() {
        let is_quiet = !mv.is_noisy();

        if (is_quiet && skip_quiets) || !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;

        if !is_root && !is_loss(best_score) {
            let lmr_depth = (depth - td.lmr.reduction(depth, move_count) / 1024).max(0);

            skip_quiets |= move_count >= lmp_threshold(depth);

            skip_quiets |= is_quiet && lmr_depth < 10 && eval + 100 * lmr_depth + 150 <= alpha;

            let threshold = if is_quiet { -30 * lmr_depth * lmr_depth } else { -95 * depth };
            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.board.make_move::<true, false>(mv);
        td.tt.prefetch(td.board.hash());

        let new_depth = depth - 1;

        let mut score = Score::ZERO;

        if depth >= 3 && move_count > 1 + is_root as i32 && is_quiet {
            let mut reduction = td.lmr.reduction(depth, move_count) / 1024;

            if td.board.in_check() {
                reduction -= 1;
            }

            if tt_pv {
                reduction -= 1;
            }

            if cut_node {
                reduction += 1;
            }

            let reduced_depth = (new_depth - reduction).max(1).min(new_depth);

            score = -search::<false>(td, -alpha - 1, -alpha, reduced_depth, true);

            if score > alpha && new_depth > reduced_depth {
                score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
            }
        } else if !PV || move_count > 1 {
            score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
        }

        if PV && (move_count == 1 || score > alpha) {
            score = -search::<true>(td, -beta, -alpha, new_depth, false);
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
                    if best_move.is_noisy() {
                        td.noisy_history.update(&td.board, best_move, &noisy_moves, depth);
                    } else {
                        td.quiet_history.update(&td.board, best_move, &quiet_moves, depth);
                    }
                    break;
                }

                alpha = score;
            }
        }

        if mv != best_move && move_count < 32 {
            if mv.is_noisy() {
                noisy_moves.push(mv);
            } else {
                quiet_moves.push(mv);
            }
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

    td.tt.write(td.board.hash(), depth, best_score, bound, best_move, td.ply, tt_pv);

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

    while let Some((mv, mv_score)) = move_picker.next() {
        if mv_score < -(1 << 18) {
            break;
        }

        if !td.board.is_legal(mv) {
            continue;
        }

        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.board.make_move::<true, false>(mv);

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
