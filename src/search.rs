use std::time::Instant;

use crate::{
    evaluate::evaluate,
    movepick::{MovePicker, Stage},
    parameters::*,
    thread::ThreadData,
    transposition::Bound,
    types::{is_decisive, is_loss, is_win, mate_in, mated_in, ArrayVec, Color, Move, Piece, Score, Square, MAX_PLY},
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Report {
    None,
    Minimal,
    Full,
}

#[derive(Copy, Clone)]
pub struct SearchResult {
    pub best_move: Move,
    pub depth: i32,
    pub score: i32,
}

pub fn start(td: &mut ThreadData, report: Report) -> SearchResult {
    td.completed_depth = 0;
    td.stopped = false;

    td.pv.clear(0);
    td.node_table.clear();
    td.counter.clear();

    td.nnue.refresh(&td.board);

    let now = Instant::now();

    let mut average = Score::NONE;
    let mut last_move = Move::NULL;

    let mut eval_stability = 0;
    let mut pv_stability = 0;

    for depth in 1..MAX_PLY as i32 {
        td.sel_depth = 0;
        td.root_depth = depth;

        let mut alpha = -Score::INFINITE;
        let mut beta = Score::INFINITE;

        let mut delta = asp_delta();
        let mut reduction = 0;

        // Aspiration Windows
        if depth >= 4 {
            delta += average * average / asp_div();

            alpha = (average - delta).max(-Score::INFINITE);
            beta = (average + delta).min(Score::INFINITE);
        }

        loop {
            td.stack = Default::default();

            td.root_delta = beta - alpha;
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
                    average = if average == Score::NONE { current } else { (average + current) / 2 };
                    break;
                }
            }

            delta += delta / 2;
        }

        if td.stopped {
            break;
        }

        td.completed_depth = depth;

        if last_move == td.pv.best_move() {
            pv_stability = (pv_stability + 1).min(8);
        } else {
            pv_stability = 0;
            last_move = td.pv.best_move();
        }

        if (td.best_score - average).abs() < 12 {
            eval_stability = (eval_stability + 1).min(8);
        } else {
            eval_stability = 0;
        }

        if td.time_manager.soft_limit(td, pv_stability, eval_stability) {
            break;
        }

        if report == Report::Full {
            td.print_uci_info(depth, td.best_score, now);
        }
    }

    if report != Report::None {
        td.print_uci_info(td.root_depth, td.best_score, now);
    }

    SearchResult {
        best_move: td.pv.best_move(),
        depth: td.completed_depth,
        score: td.best_score,
    }
}

fn search<const PV: bool>(td: &mut ThreadData, mut alpha: i32, mut beta: i32, depth: i32, cut_node: bool) -> i32 {
    debug_assert!(td.ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let is_root = td.ply == 0;
    let in_check = td.board.in_check();
    let excluded = td.stack[td.ply].excluded.is_valid();

    td.pv.clear(td.ply);

    if td.stopped {
        return Score::ZERO;
    }

    if !is_root && alpha < Score::ZERO && td.board.upcoming_repetition() {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    if depth <= 0 {
        return qsearch::<PV>(td, alpha, beta);
    }

    td.counter.increment();

    if PV {
        td.sel_depth = td.sel_depth.max(td.ply as i32 + 1);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if !is_root {
        if td.board.is_draw() {
            return Score::DRAW;
        }

        if td.ply >= MAX_PLY - 1 {
            return if in_check { Score::DRAW } else { evaluate(td) };
        }

        // Mate Distance Pruning (MDP)
        alpha = alpha.max(mated_in(td.ply));
        beta = beta.min(mate_in(td.ply + 1));

        if alpha >= beta {
            return alpha;
        }
    }

    let mut depth = depth.min(MAX_PLY as i32 - 1);

    let entry = td.tt.read(td.board.hash(), td.ply);
    let mut tt_move = Move::NULL;
    let mut tt_pv = PV;

    if let Some(entry) = entry {
        tt_move = entry.mv;
        tt_pv |= entry.pv;

        if !PV
            && !excluded
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

    let correction_value = correction_value(td);

    let raw_eval;
    let static_eval;
    let mut eval;

    if in_check {
        raw_eval = Score::NONE;
        static_eval = Score::NONE;
        eval = Score::NONE;
    } else if excluded {
        raw_eval = td.stack[td.ply].static_eval;
        static_eval = raw_eval;
        eval = static_eval;
    } else if let Some(entry) = entry {
        raw_eval = if entry.eval != Score::NONE { entry.eval } else { evaluate(td) };
        static_eval = corrected_eval(raw_eval, correction_value, td.board.halfmove_clock());
        eval = static_eval;

        if match entry.bound {
            Bound::Upper => entry.score < eval,
            Bound::Lower => entry.score > eval,
            _ => true,
        } {
            eval = entry.score;
        }
    } else {
        raw_eval = evaluate(td);
        static_eval = corrected_eval(raw_eval, correction_value, td.board.halfmove_clock());
        eval = static_eval;
    }

    if !in_check
        && !excluded
        && td.ply >= 1
        && td.stack[td.ply - 1].mv.is_valid()
        && td.stack[td.ply - 1].mv.is_quiet()
        && td.stack[td.ply - 1].static_eval != Score::NONE
    {
        let value = 6 * -(static_eval + td.stack[td.ply - 1].static_eval);
        let bonus = value.clamp(-64, 128);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[td.ply - 1].mv, bonus);
    }

    let improving = !in_check
        && td.ply >= 2
        && td.stack[td.ply - 1].mv.is_valid()
        && static_eval > td.stack[td.ply - 2].static_eval;

    if td.ply >= 1 && td.stack[td.ply - 1].reduction >= 3072 && static_eval + td.stack[td.ply - 1].static_eval < 0 {
        depth += 1;
    }

    if !in_check
        && depth >= 2
        && td.ply >= 1
        && td.stack[td.ply - 1].reduction >= 1024
        && td.stack[td.ply - 1].static_eval != Score::NONE
        && static_eval + td.stack[td.ply - 1].static_eval > 96
    {
        depth -= 1;
    }

    td.stack[td.ply].static_eval = static_eval;
    td.stack[td.ply].tt_pv = tt_pv;

    td.stack[td.ply + 1].killer = Move::NULL;
    td.stack[td.ply + 2].cutoff_count = 0;

    // Razoring
    if !PV && !in_check && eval < alpha - 300 - 250 * depth * depth {
        return qsearch::<false>(td, alpha, beta);
    }

    // Reverse Futility Pruning (RFP)
    if !PV
        && !in_check
        && !excluded
        && depth <= 8
        && eval >= beta
        && eval
            >= beta + 80 * depth - (80 * improving as i32) - (30 * cut_node as i32) + correction_value.abs() / 2 + 25
    {
        return ((eval + beta) / 2).clamp(-16384, 16384);
    }

    // Null Move Pruning (NMP)
    if cut_node
        && !in_check
        && !excluded
        && eval >= beta
        && eval >= static_eval
        && static_eval
            >= beta - 20 * depth + 128 * tt_pv as i32 + 180
                - td.nmp_history.get(td.board.side_to_move(), td.stack[td.ply - 1].mv) / 16
        && td.ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
    {
        let r = 4 + depth / 3 + ((eval - beta) / 256).min(3) + (tt_move.is_null() || tt_move.is_noisy()) as i32;

        td.stack[td.ply].piece = Piece::None;
        td.stack[td.ply].mv = Move::NULL;
        td.ply += 1;

        td.board.make_null_move();

        let mut score = -search::<false>(td, -beta, -beta + 1, depth - r, false);

        td.board.undo_null_move();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        let bonus = if score >= beta { stat_bonus(depth) / 2 } else { -stat_bonus(depth) };
        td.nmp_history.update(td.board.side_to_move(), td.stack[td.ply - 1].mv, bonus);

        if score >= beta {
            if is_win(score) {
                score = beta;
            }

            if td.nmp_min_ply > 0 || depth < 16 {
                return score;
            }

            td.nmp_min_ply = td.ply as i32 + 3 * (depth - r) / 4;
            let verified_score = search::<false>(td, beta - 1, beta, depth - r, false);
            td.nmp_min_ply = 0;

            if td.stopped {
                return Score::ZERO;
            }

            if verified_score >= beta {
                return score;
            }
        }
    }

    // ProbCut
    let probcut_beta = beta + 256 - 64 * improving as i32;

    if depth >= 3 && !is_decisive(beta) && entry.is_none_or(|entry| entry.score >= probcut_beta) {
        let mut move_picker = MovePicker::new_probcut(probcut_beta - static_eval);

        let probcut_depth = 0.max(depth - 4);

        while let Some(mv) = move_picker.next(td, true) {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if mv == td.stack[td.ply].excluded || !td.board.is_legal(mv) {
                continue;
            }

            td.stack[td.ply].piece = td.board.moved_piece(mv);
            td.stack[td.ply].mv = mv;
            td.ply += 1;

            td.nnue.push(mv, &td.board);
            td.board.make_move(mv);
            td.tt.prefetch(td.board.hash());

            let mut score = -qsearch::<false>(td, -probcut_beta, -probcut_beta + 1);

            if score >= probcut_beta && probcut_depth > 0 {
                score = -search::<false>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, !cut_node);
            }

            td.board.undo_move(mv);
            td.nnue.pop();
            td.ply -= 1;

            if td.stopped {
                return Score::ZERO;
            }

            if score >= probcut_beta {
                td.tt.write(td.board.hash(), probcut_depth + 1, raw_eval, score, Bound::Lower, mv, td.ply, tt_pv);

                if is_decisive(score) {
                    return score;
                }

                return score - (probcut_beta - beta);
            }
        }
    }

    // Internal Iterative Reductions (IIR)
    if depth >= 3 + 3 * cut_node as i32 && tt_move.is_null() && (PV || cut_node) {
        depth -= 1;
    }

    let mut best_score = -Score::INFINITE;
    let mut best_move = Move::NULL;
    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(td.stack[td.ply].killer, tt_move);
    let mut skip_quiets = false;

    while let Some(mv) = move_picker.next(td, skip_quiets) {
        if mv == td.stack[td.ply].excluded || !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;

        let is_quiet = mv.is_quiet();

        let history = if is_quiet {
            td.quiet_history.get(td.board.threats(), td.board.side_to_move(), mv)
                + td.conthist(1, mv)
                + td.conthist(2, mv)
        } else {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.get(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured)
        };

        let mut reduction = td.lmr.reduction(depth, move_count);

        if !is_root && !is_loss(best_score) {
            let lmr_depth = (depth - reduction / 1024).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= move_count >= lmp_threshold(depth, improving);

            // Futility Pruning (FP)
            skip_quiets |= !in_check && is_quiet && lmr_depth < 10 && static_eval + 100 * lmr_depth + 150 <= alpha;

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet { -30 * lmr_depth * lmr_depth } else { -95 * depth + 50 } - history / 32;
            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        // Singular Extensions (SE)
        let mut extension = 0;

        if !is_root && !excluded && td.ply < 2 * td.root_depth as usize && mv == tt_move {
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
                    extension += (!PV && score < singular_beta - 24) as i32;
                    extension += (!PV && is_quiet && score < singular_beta - 128) as i32;
                    if extension > 1 && depth < 12 {
                        depth += 1;
                    }
                } else if score >= beta {
                    return score;
                } else if entry.score >= beta {
                    extension = -2;
                } else if cut_node {
                    extension = -2;
                }
            }
        }

        let initial_nodes = td.counter.local();

        td.stack[td.ply].piece = td.board.moved_piece(mv);
        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.nnue.push(mv, &td.board);
        td.board.make_move(mv);
        td.tt.prefetch(td.board.hash());

        let mut new_depth = depth + extension - 1;
        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 3 && move_count > 1 + is_root as i32 && (is_quiet || !tt_pv) {
            if tt_pv {
                reduction -= 1536;
                reduction -= 120 * entry.map_or(0, |entry| entry.depth - depth) as i32;
                reduction -= 3 * entry.map_or(0, |entry| entry.score - alpha);
            }

            if PV {
                reduction -= 768 + 768 * (beta - alpha > td.root_delta / 4) as i32;
            }

            if cut_node {
                reduction += 1024;
            }

            reduction -= 4 * correction_value.abs();

            if td.board.in_check() {
                reduction -= 1024;
            }

            if !improving {
                reduction += 1024;
            }

            if td.stack[td.ply].cutoff_count > 2 {
                reduction += 896 + 64 * td.stack[td.ply].cutoff_count.max(8);
            }

            if td.stack[td.ply - 1].killer == mv {
                reduction -= 1024;
            }

            reduction -= (history - 512) / 16;

            let reduced_depth = (new_depth - reduction / 1024).clamp(0, new_depth);

            td.stack[td.ply - 1].reduction = reduction;

            score = -search::<false>(td, -alpha - 1, -alpha, reduced_depth, true);

            td.stack[td.ply - 1].reduction = 0;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 64) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
                }

                let mut bonus = match score {
                    s if s >= beta => (1 + 2 * (move_count > depth) as i32) * stat_bonus(depth),
                    s if s <= alpha => -stat_bonus(depth),
                    _ => 0,
                };

                if !is_quiet {
                    bonus /= 5;
                }

                td.ply -= 1;
                update_continuation_histories(td, td.stack[td.ply].piece, mv.to(), bonus);
                td.ply += 1;
            }
        }
        // Full Depth Search (FDS)
        else if !PV || move_count > 1 {
            score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
        }

        // Principal Variation Search (PVS)
        if PV && (move_count == 1 || score > alpha) {
            score = -search::<true>(td, -beta, -alpha, new_depth, false);
        }

        td.board.undo_move(mv);
        td.nnue.pop();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if is_root {
            td.node_table.add(mv, td.counter.local() - initial_nodes);
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                bound = Bound::Exact;
                alpha = score;
                best_move = mv;

                if PV {
                    td.pv.update(td.ply, mv);

                    if is_root {
                        td.best_score = score;
                    }
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

    if best_move.is_valid() {
        let bonus = stat_bonus(depth);
        let malus = stat_bonus(depth) - 16 * (move_count - 1);

        if best_move.is_noisy() {
            td.noisy_history.update(
                td.board.threats(),
                td.board.moved_piece(best_move),
                best_move.to(),
                td.board.piece_on(best_move.to()).piece_type(),
                bonus,
            );

            for &mv in noisy_moves.iter() {
                let captured = td.board.piece_on(mv.to()).piece_type();
                td.noisy_history.update(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured, -malus);
            }
        } else {
            td.stack[td.ply].killer = best_move;

            if !quiet_moves.is_empty() || depth > 3 {
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), best_move, bonus);
                update_continuation_histories(td, td.board.moved_piece(best_move), best_move.to(), bonus);
            }

            for &mv in quiet_moves.iter() {
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), mv, -malus);
            }

            for &mv in noisy_moves.iter() {
                let captured = td.board.piece_on(mv.to()).piece_type();
                td.noisy_history.update(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured, -malus);
            }

            for &mv in quiet_moves.iter() {
                update_continuation_histories(td, td.board.moved_piece(mv), mv.to(), -malus);
            }
        }
    }

    if bound == Bound::Upper && td.ply >= 1 && depth > 3 {
        td.ply -= 1;
        tt_pv |= td.stack[td.ply].tt_pv;

        let factor = 2;
        let scaled_bonus = factor * stat_bonus(depth);

        let pcm_move = td.stack[td.ply].mv;
        if pcm_move != Move::NULL && pcm_move.is_quiet() {
            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);
        }
        td.ply += 1;
    }

    if !excluded {
        td.tt.write(td.board.hash(), depth, raw_eval, best_score, bound, best_move, td.ply, tt_pv);
    }

    if !(in_check
        || best_move.is_noisy()
        || is_decisive(best_score)
        || (bound == Bound::Upper && best_score >= static_eval)
        || (bound == Bound::Lower && best_score <= static_eval))
    {
        update_correction_histories(td, depth, best_score - static_eval);
    }

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn qsearch<const PV: bool>(td: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    debug_assert!(td.ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let in_check = td.board.in_check();

    td.counter.increment();

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.ply >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { evaluate(td) };
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

    let mut best_score = -Score::INFINITE;
    let mut futility_score = Score::NONE;
    let mut raw_eval = Score::NONE;

    if !in_check {
        raw_eval = match entry {
            Some(entry) if entry.eval != Score::NONE => entry.eval,
            _ => evaluate(td),
        };

        let static_eval = corrected_eval(raw_eval, correction_value(td), td.board.halfmove_clock());
        best_score = static_eval;

        if let Some(entry) = entry {
            if match entry.bound {
                Bound::Upper => entry.score < static_eval,
                Bound::Lower => entry.score > static_eval,
                _ => true,
            } {
                best_score = entry.score;
            }
        }

        if best_score >= beta {
            if entry.is_none() {
                td.tt.write(td.board.hash(), 0, raw_eval, best_score, Bound::Lower, Move::NULL, td.ply, tt_pv);
            }

            return best_score;
        }

        if best_score > alpha {
            alpha = best_score;
        }

        futility_score = static_eval + 128;
    }

    let mut best_move = Move::NULL;

    let mut move_count = 0;
    let mut move_picker = MovePicker::new_qsearch();

    let previous_square = match td.stack[td.ply - 1].mv {
        Move::NULL => Square::None,
        _ => td.stack[td.ply - 1].mv.to(),
    };

    while let Some(mv) = move_picker.next(td, !in_check) {
        if !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;

        if !is_loss(best_score) && mv.to() != previous_square {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if move_count >= 3 {
                break;
            }

            if mv.is_quiet() {
                continue;
            }

            if !in_check && futility_score <= alpha && !td.board.see(mv, 1) {
                best_score = best_score.max(futility_score);
                continue;
            }
        }

        td.stack[td.ply].piece = td.board.moved_piece(mv);
        td.stack[td.ply].mv = mv;
        td.ply += 1;

        td.nnue.push(mv, &td.board);
        td.board.make_move(mv);

        let score = -qsearch::<PV>(td, -beta, -alpha);

        td.board.undo_move(mv);
        td.nnue.pop();
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

    if in_check && move_count == 0 {
        return mated_in(td.ply);
    }

    if best_score >= beta && !is_decisive(best_score) && !is_decisive(beta) {
        best_score = (best_score + beta) / 2;
    }

    let bound = if best_score >= beta { Bound::Lower } else { Bound::Upper };

    td.tt.write(td.board.hash(), 0, raw_eval, best_score, bound, best_move, td.ply, tt_pv);

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn correction_value(td: &ThreadData) -> i32 {
    let stm = td.board.side_to_move();

    td.pawn_corrhist.get(stm, td.board.pawn_key())
        + td.minor_corrhist.get(stm, td.board.minor_key())
        + td.major_corrhist.get(stm, td.board.major_key())
        + td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black))
        + if td.ply >= 1 { td.last_move_corrhist.get(stm, td.stack[td.ply - 1].mv.encoded() as u64) } else { 0 }
}

fn corrected_eval(eval: i32, correction_value: i32, hmr: u8) -> i32 {
    (eval * (200 - hmr as i32) / 200 + correction_value).clamp(-16384, 16384)
}

fn stat_bonus(depth: i32) -> i32 {
    (128 * depth - 64).min(1280)
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32) {
    let stm = td.board.side_to_move();

    td.pawn_corrhist.update(stm, td.board.pawn_key(), depth, diff);
    td.minor_corrhist.update(stm, td.board.minor_key(), depth, diff);
    td.major_corrhist.update(stm, td.board.major_key(), depth, diff);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), depth, diff);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), depth, diff);

    if td.ply >= 1 && td.stack[td.ply - 1].mv.is_valid() {
        td.last_move_corrhist.update(td.board.side_to_move(), td.stack[td.ply - 1].mv.encoded() as u64, depth, diff);
    }
}

fn update_continuation_histories(td: &mut ThreadData, piece: Piece, sq: Square, bonus: i32) {
    if td.ply >= 1 {
        let entry = td.stack[td.ply - 1];
        if entry.mv.is_valid() {
            td.continuation_history.update(entry.piece, entry.mv.to(), piece, sq, bonus);
        }
    }

    if td.ply >= 2 {
        let entry = td.stack[td.ply - 2];
        if entry.mv.is_valid() {
            td.continuation_history.update(entry.piece, entry.mv.to(), piece, sq, bonus);
        }
    }
}
