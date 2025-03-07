use std::time::Instant;

use crate::{
    movepick::MovePicker,
    parameters::*,
    thread::ThreadData,
    transposition::Bound,
    types::{is_decisive, is_loss, mated_in, ArrayVec, Move, Piece, Score, MAX_PLY},
};

pub fn start(td: &mut ThreadData, silent: bool) {
    td.nodes = 0;
    td.completed_depth = 0;
    td.stopped = false;
    td.pv.clear(0);
    td.node_table.clear();

    let now = Instant::now();
    let mut score = Score::NONE;

    for depth in 1..MAX_PLY as i32 {
        let mut alpha = -Score::INFINITE;
        let mut beta = Score::INFINITE;

        let mut delta = asp_delta();
        let mut reduction = 0;

        if depth >= 4 {
            delta += score * score / asp_div();

            alpha = (score - delta).max(-Score::INFINITE);
            beta = (score + delta).min(Score::INFINITE);
        }

        loop {
            let current = search::<true>(td, alpha, beta, (depth - reduction).max(1), false);

            if td.stopped {
                break;
            }

            match current {
                s if s <= alpha => {
                    beta = (alpha + beta) / 2;
                    alpha = (current - delta).max(-Score::INFINITE);
                    reduction = 0;
                }
                s if s >= beta => {
                    beta = (current + delta).min(Score::INFINITE);
                    reduction += 1;
                }
                _ => {
                    score = current;
                    break;
                }
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

        if td.time_manager.soft_limit(td) {
            break;
        }
    }
}

fn search<const PV: bool>(td: &mut ThreadData, mut alpha: i32, beta: i32, depth: i32, cut_node: bool) -> i32 {
    let is_root = td.ply == 0;
    let in_check = td.board.in_check();
    let excluded = td.stack[td.ply].excluded != Move::NULL;

    td.pv.clear(td.ply);

    if depth <= 0 && !in_check {
        return qsearch::<PV>(td, alpha, beta);
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

    let entry = if excluded { None } else { td.tt.read(td.board.hash(), td.ply) };
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

    let static_eval;
    let mut eval;

    if in_check {
        static_eval = Score::NONE;
        eval = Score::NONE;
    } else if excluded {
        static_eval = td.stack[td.ply].eval;
        eval = static_eval;
    } else {
        static_eval = td.board.evaluate() + correction_value(td);
        eval = static_eval;

        if let Some(entry) = entry {
            if match entry.bound {
                Bound::Upper => entry.score < eval,
                Bound::Lower => entry.score > eval,
                _ => true,
            } {
                eval = entry.score;
            }
        }
    }

    let improving = !in_check && td.ply >= 2 && static_eval > td.stack[td.ply - 2].eval;

    td.stack[td.ply].eval = static_eval;
    td.stack[td.ply].tt_pv = tt_pv;
    td.stack[td.ply].multiple_extensions = if is_root { 0 } else { td.stack[td.ply - 1].multiple_extensions };

    td.stack[td.ply + 2].cutoff_count = 0;

    if !PV && eval < alpha - 300 - 250 * depth * depth {
        return qsearch::<false>(td, alpha, beta);
    }

    if !PV && !in_check && !excluded && depth <= 8 && eval >= beta + 80 * depth - (80 * improving as i32) {
        return eval;
    }

    if !PV
        && !in_check
        && !excluded
        && depth >= 3
        && eval >= beta
        && static_eval >= beta - 20 * depth + 180
        && td.stack[td.ply - 1].mv != Move::NULL
        && td.board.has_non_pawns()
    {
        let r = 3 + depth / 3 + ((eval - beta) / 256).min(3);

        td.stack[td.ply].piece = Piece::None;
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
    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(td, tt_move);
    let mut skip_quiets = false;

    while let Some((mv, _)) = move_picker.next() {
        let is_quiet = !mv.is_noisy();

        if (is_quiet && skip_quiets) || mv == td.stack[td.ply].excluded || !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;

        if !is_root && !is_loss(best_score) {
            let lmr_depth = (depth - td.lmr.reduction(depth, move_count) / 1024).max(0);

            skip_quiets |= move_count >= lmp_threshold(depth);

            skip_quiets |= is_quiet && lmr_depth < 10 && static_eval + 100 * lmr_depth + 150 <= alpha;

            let threshold = if is_quiet { -30 * lmr_depth * lmr_depth } else { -95 * depth };
            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        let mut extension = 0;

        if !is_root && !excluded && mv == tt_move {
            let entry = entry.unwrap();

            if depth >= 8 && entry.depth >= depth - 3 && entry.bound != Bound::Upper && !is_decisive(entry.score) {
                let singular_beta = entry.score - depth;
                let singular_depth = (depth - 1) / 2;

                td.stack[td.ply].excluded = entry.mv;
                let score = search::<false>(td, singular_beta - 1, singular_beta, singular_depth, cut_node);
                td.stack[td.ply].excluded = Move::NULL;

                if td.stopped {
                    return Score::ZERO;
                }

                if score < singular_beta {
                    extension = 1;
                    extension += (!PV && score <= singular_beta - 24) as i32;

                    td.stack[td.ply].multiple_extensions += (extension > 1) as i32;
                } else if singular_beta >= beta {
                    return singular_beta;
                }
            }
        }

        let initial_nodes = td.nodes;
        let mut new_depth = depth + extension - 1;

        td.stack[td.ply].piece = td.board.piece_on(mv.from());
        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.board.make_move::<true, false>(mv);
        td.tt.prefetch(td.board.hash());

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

            if !improving {
                reduction += 1;
            }

            if td.stack[td.ply].cutoff_count > 3 {
                reduction += 1;
            }

            let reduced_depth = (new_depth - reduction).max(1).min(new_depth);

            score = -search::<false>(td, -alpha - 1, -alpha, reduced_depth, true);

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 64) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
                }
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

        if is_root {
            td.node_table.add(mv, td.nodes - initial_nodes);
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                bound = Bound::Exact;
                alpha = score;
                best_move = mv;

                if PV {
                    td.pv.update(td.ply, mv);
                }

                if score >= beta {
                    bound = Bound::Lower;
                    td.stack[td.ply].cutoff_count += 1;
                    break;
                }
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
        if excluded {
            return alpha;
        }

        return if in_check { mated_in(td.ply) } else { Score::DRAW };
    }

    if bound == Bound::Lower {
        let bonus = bonus(depth);

        if best_move.is_noisy() {
            td.noisy_history.update(&td.board, best_move, bonus);

            for &mv in noisy_moves.iter() {
                td.noisy_history.update(&td.board, mv, -bonus);
            }
        } else {
            td.quiet_history.update(&td.board, best_move, bonus);

            for &mv in quiet_moves.iter() {
                td.quiet_history.update(&td.board, mv, -bonus);
            }

            if td.ply >= 1 && td.stack[td.ply - 1].mv != Move::NULL {
                let prev_mv = td.stack[td.ply - 1].mv;
                let prev_piece = td.stack[td.ply - 1].piece;

                td.continuation_history.update(&td.board, prev_mv, prev_piece, best_move, bonus);

                for &mv in quiet_moves.iter() {
                    td.continuation_history.update(&td.board, prev_mv, prev_piece, mv, -bonus);
                }
            }
        }
    }

    if bound == Bound::Upper {
        tt_pv |= td.ply >= 1 && td.stack[td.ply - 1].tt_pv;
    }

    if !excluded {
        td.tt.write(td.board.hash(), depth, best_score, bound, best_move, td.ply, tt_pv);
    }

    if !(excluded
        || in_check
        || best_move.is_noisy()
        || is_decisive(best_score)
        || (bound == Bound::Upper && best_score >= static_eval)
        || (bound == Bound::Lower && best_score <= static_eval))
    {
        td.pawn_corrhist.update(td.board.side_to_move(), td.board.pawn_key(), depth, best_score - static_eval);
        td.minor_corrhist.update(td.board.side_to_move(), td.board.minor_key(), depth, best_score - static_eval);
        td.major_corrhist.update(td.board.side_to_move(), td.board.major_key(), depth, best_score - static_eval);
    }

    best_score
}

fn qsearch<const PV: bool>(td: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    let in_check = td.board.in_check();

    td.nodes += 1;

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.ply >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { td.board.evaluate() };
    }

    let entry = td.tt.read(td.board.hash(), td.ply);
    let mut tt_pv = PV;

    if let Some(entry) = entry {
        tt_pv |= entry.pv;
        if match entry.bound {
            Bound::Upper => entry.score <= alpha,
            Bound::Lower => entry.score >= beta,
            _ => true,
        } {
            return entry.score;
        }
    }

    let mut best_score = td.board.evaluate() + correction_value(td);

    if best_score >= beta {
        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let mut best_move = Move::NULL;
    let mut move_picker = MovePicker::new_noisy(td);

    while let Some((mv, mv_score)) = move_picker.next() {
        if mv_score < -(1 << 18) {
            break;
        }

        if !td.board.is_legal(mv) {
            continue;
        }

        td.stack[td.ply].piece = td.board.piece_on(mv.from());
        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.board.make_move::<true, false>(mv);

        let score = -qsearch::<PV>(td, -beta, -alpha);

        td.board.undo_move::<true>(mv);
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = mv;
                alpha = score;

                if score >= beta {
                    break;
                }
            }
        }
    }

    let bound = if best_score >= beta { Bound::Lower } else { Bound::Upper };

    td.tt.write(td.board.hash(), 0, best_score, bound, best_move, td.ply, tt_pv);

    best_score
}

fn correction_value(td: &ThreadData) -> i32 {
    let stm = td.board.side_to_move();

    td.pawn_corrhist.get(stm, td.board.pawn_key())
        + td.minor_corrhist.get(stm, td.board.minor_key())
        + td.major_corrhist.get(stm, td.board.major_key())
}

fn bonus(depth: i32) -> i32 {
    (128 * depth - 64).min(1280)
}
