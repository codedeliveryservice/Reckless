use std::time::Instant;

use crate::{
    evaluate::evaluate,
    movepick::{MovePicker, Stage},
    parameters::*,
    tb::{tb_probe, tb_size, GameOutcome},
    thread::ThreadData,
    transposition::{Bound, TtDepth},
    types::{
        is_decisive, is_loss, is_valid, is_win, mate_in, mated_in, tb_loss_in, tb_win_in, ArrayVec, Color, Move, Piece,
        Score, Square, MAX_PLY,
    },
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
    td.tb_hits.clear();

    td.nnue.refresh(&td.board);

    let now = Instant::now();

    let mut average = Score::NONE;
    let mut last_move = Move::NULL;

    let mut eval_stability = 0;
    let mut pv_stability = 0;

    let mut window_expansion = 0;

    // Iterative Deepening
    for depth in 1..MAX_PLY as i32 {
        td.sel_depth = 0;
        td.root_depth = depth;

        let mut alpha = -Score::INFINITE;
        let mut beta = Score::INFINITE;

        let mut delta = 10;
        let mut reduction = 0;

        // Aspiration Windows
        if depth >= 4 {
            delta += window_expansion + average * average / 27874;

            alpha = (average - delta).max(-Score::INFINITE);
            beta = (average + delta).min(Score::INFINITE);

            td.optimism[td.board.side_to_move()] = 128 * average / (average.abs() + 212);
            td.optimism[!td.board.side_to_move()] = -td.optimism[td.board.side_to_move()];
        }

        loop {
            td.stack = Default::default();
            td.root_delta = beta - alpha;

            // Root Search
            let score = search::<true>(td, alpha, beta, (depth - reduction).max(1), false);

            if td.stopped {
                break;
            }

            match score {
                s if s <= alpha => {
                    window_expansion += 1;
                    beta = (alpha + beta) / 2;
                    alpha = (score - delta).max(-Score::INFINITE);
                    reduction = 0;
                }
                s if s >= beta => {
                    window_expansion += 1;
                    beta = (score + delta).min(Score::INFINITE);
                    reduction += 1;
                }
                _ => {
                    window_expansion /= 2;
                    average = if average == Score::NONE { score } else { (average + score) / 2 };
                    break;
                }
            }

            delta += delta * (45 + 14 * reduction) / 128;
        }

        if td.stopped {
            break;
        }

        td.counter.flush();
        td.tb_hits.flush();
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
    let excluded = td.stack[td.ply].excluded.is_some();

    if PV {
        td.pv.clear(td.ply);
    }

    if td.stopped {
        return Score::ZERO;
    }

    if !is_root && alpha < Score::ZERO && td.board.upcoming_repetition(td.ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    // Qsearch Dive
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
        if td.board.is_draw(td.ply) {
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

    let mut best_score = -Score::INFINITE;
    let mut max_score = Score::INFINITE;

    let mut depth = depth.min(MAX_PLY as i32 - 1);

    let entry = td.tt.read(td.board.hash(), td.board.halfmove_clock(), td.ply);
    let mut tt_depth = 0;
    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    let mut tt_pv = PV;

    // Search Early TT-Cut
    if let Some(entry) = entry {
        tt_move = entry.mv;
        tt_pv |= entry.pv;
        tt_score = entry.score;
        tt_depth = entry.depth;
        tt_bound = entry.bound;

        if !PV
            && !excluded
            && tt_depth >= depth
            && is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha,
                Bound::Lower => tt_score >= beta,
                _ => true,
            }
        {
            if tt_move.is_some() && tt_move.is_quiet() && tt_score >= beta {
                let bonus = (124 * depth - 64).min(1367);
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), tt_move, bonus);
                update_continuation_histories(td, td.board.moved_piece(tt_move), tt_move.to(), bonus);
            }

            if td.board.halfmove_clock() < 90 {
                debug_assert!(is_valid(tt_score));
                return tt_score;
            }
        }
    }

    // Tablebases Probe
    if !is_root
        && !excluded
        && td.board.halfmove_clock() == 0
        && td.board.castling().raw() == 0
        && td.board.occupancies().len() <= tb_size()
    {
        if let Some(outcome) = tb_probe(&td.board) {
            td.tb_hits.increment();

            let (score, bound) = match outcome {
                GameOutcome::Win => (tb_win_in(td.ply), Bound::Lower),
                GameOutcome::Loss => (tb_loss_in(td.ply), Bound::Upper),
                GameOutcome::Draw => (Score::DRAW, Bound::Exact),
            };

            if bound == Bound::Exact
                || (bound == Bound::Lower && score >= beta)
                || (bound == Bound::Upper && score <= alpha)
            {
                let depth = (depth + 6).min(MAX_PLY as i32 - 1);
                td.tt.write(td.board.hash(), depth, Score::NONE, score, bound, Move::NULL, td.ply, tt_pv);
                return score;
            }

            if PV {
                if bound == Bound::Lower {
                    best_score = score;
                    alpha = alpha.max(best_score);
                } else {
                    max_score = score;
                }
            }
        }
    }

    let correction_value = correction_value(td);

    let raw_eval;
    let static_eval;
    let mut eval;

    // Evaluation
    if in_check {
        raw_eval = Score::NONE;
        static_eval = Score::NONE;
        eval = Score::NONE;
    } else if excluded {
        raw_eval = td.stack[td.ply].static_eval;
        static_eval = raw_eval;
        eval = static_eval;
    } else if let Some(entry) = entry {
        raw_eval = if is_valid(entry.eval) { entry.eval } else { evaluate(td) };
        static_eval = corrected_eval(raw_eval, correction_value, td.board.halfmove_clock());
        eval = static_eval;

        if is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score < eval,
                Bound::Lower => tt_score > eval,
                _ => true,
            }
        {
            debug_assert!(is_valid(tt_score));
            eval = tt_score;
        }
    } else {
        raw_eval = evaluate(td);
        td.tt.write(td.board.hash(), TtDepth::SOME, raw_eval, Score::NONE, Bound::None, Move::NULL, td.ply, tt_pv);

        static_eval = corrected_eval(raw_eval, correction_value, td.board.halfmove_clock());
        eval = static_eval;
    }

    td.stack[td.ply].static_eval = static_eval;
    td.stack[td.ply].tt_pv = tt_pv;

    td.stack[td.ply + 1].killer = Move::NULL;
    td.stack[td.ply + 2].cutoff_count = 0;

    // Quiet Move Ordering Using Static-Eval
    if !in_check
        && !excluded
        && td.ply >= 1
        && td.stack[td.ply - 1].mv.is_some()
        && td.stack[td.ply - 1].mv.is_quiet()
        && is_valid(td.stack[td.ply - 1].static_eval)
    {
        let value = 664 * (-(static_eval + td.stack[td.ply - 1].static_eval)) / 128;
        let bonus = value.clamp(-69, 164);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[td.ply - 1].mv, bonus);
    }

    // Hindsight LMR
    if !in_check
        && !excluded
        && td.ply >= 1
        && td.stack[td.ply - 1].reduction >= 2551
        && static_eval + td.stack[td.ply - 1].static_eval < 0
    {
        depth += 1;
    }

    if !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.ply >= 1
        && td.stack[td.ply - 1].reduction >= 1014
        && is_valid(td.stack[td.ply - 1].static_eval)
        && static_eval + td.stack[td.ply - 1].static_eval > 67
    {
        depth -= 1;
    }

    // Hindsight Late TT-Cut
    if !PV
        && !excluded
        && tt_depth >= depth
        && td.board.halfmove_clock() < 90
        && is_valid(tt_score)
        && match tt_bound {
            Bound::Upper => tt_score <= alpha,
            Bound::Lower => tt_score >= beta,
            _ => true,
        }
    {
        debug_assert!(is_valid(tt_score));
        return tt_score;
    }

    let improving =
        !in_check && td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && static_eval > td.stack[td.ply - 2].static_eval;

    // Razoring
    if !PV && !in_check && eval < alpha - 268 - 250 * depth * depth {
        return qsearch::<false>(td, alpha, beta);
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && !in_check
        && !excluded
        && depth <= 7
        && eval >= beta
        && eval
            >= beta + 82 * depth - (69 * improving as i32) - (27 * cut_node as i32)
                + 531 * correction_value.abs() / 1024
                + 24
    {
        return ((eval + beta) / 2).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1);
    }

    // Null Move Pruning (NMP)
    if cut_node
        && !in_check
        && !excluded
        && eval >= beta
        && eval >= static_eval
        && static_eval >= beta - 15 * depth + 153 * tt_pv as i32 + 190
        && td.ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
    {
        let r = 4 + depth / 3 + ((eval - beta) / 252).min(3) + (tt_move.is_null() || tt_move.is_noisy()) as i32;

        td.stack[td.ply].piece = Piece::None;
        td.stack[td.ply].mv = Move::NULL;
        td.ply += 1;

        td.board.make_null_move();

        td.stack[td.ply].reduction = 1024 * (r - 1);
        let mut score = -search::<false>(td, -beta, -beta + 1, depth - r, false);
        td.stack[td.ply].reduction = 0;

        td.board.undo_null_move();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score >= beta {
            if is_win(score) {
                score = beta;
            }

            if td.nmp_min_ply > 0 || depth < 16 {
                return score;
            }

            td.nmp_min_ply = td.ply as i32 + 3 * (depth - r) / 4;
            td.stack[td.ply].reduction = 1024 * (r - 1);
            let verified_score = search::<false>(td, beta - 1, beta, depth - r, false);
            td.stack[td.ply].reduction = 0;
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
    let probcut_beta = beta + 298 - 64 * improving as i32;

    if depth >= 3 && !is_decisive(beta) && (!is_valid(tt_score) || tt_score >= probcut_beta) {
        let mut move_picker = MovePicker::new_probcut(probcut_beta - static_eval);

        let probcut_depth = 0.max(depth - 4);

        while let Some(mv) = move_picker.next(td, true) {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if mv == td.stack[td.ply].excluded || !td.board.is_legal(mv) {
                continue;
            }

            make_move(td, mv);

            let mut score = -qsearch::<false>(td, -probcut_beta, -probcut_beta + 1);

            if score >= probcut_beta && probcut_depth > 0 {
                td.stack[td.ply].reduction = 1024 * (depth - 1 - probcut_depth);
                score = -search::<false>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, !cut_node);
                td.stack[td.ply].reduction = 0;
            }

            undo_move(td, mv);

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

    let initial_depth = depth;

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
            let lmr_depth = (depth - reduction / 1024 + is_quiet as i32 * history / 7084).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= move_count >= lmp_threshold(depth, improving);

            // Futility Pruning (FP)
            skip_quiets |= !in_check && is_quiet && lmr_depth < 9 && static_eval + 97 * lmr_depth + 175 <= alpha;

            // Bad Noisy Futility Pruning (BNFP)
            if !in_check
                && lmr_depth < 6
                && move_picker.stage() == Stage::BadNoisy
                && static_eval + 122 * lmr_depth + 371 * move_count / 128 <= alpha
            {
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet { -21 * lmr_depth * lmr_depth } else { -98 * depth + 50 } - 42 * history / 1024;
            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        // Singular Extensions (SE)
        let mut extension = 0;

        if !is_root && !excluded && td.ply < 2 * td.root_depth as usize && mv == tt_move {
            let entry = entry.unwrap();

            if depth >= 5
                && tt_depth >= depth - 3
                && tt_bound != Bound::Upper
                && is_valid(tt_score)
                && !is_decisive(tt_score)
            {
                debug_assert!(is_valid(tt_score));
                let singular_beta = tt_score - depth;
                let singular_depth = (depth - 1) / 2;

                td.stack[td.ply].excluded = entry.mv;
                let score = search::<false>(td, singular_beta - 1, singular_beta, singular_depth, cut_node);
                td.stack[td.ply].excluded = Move::NULL;

                if td.stopped {
                    return Score::ZERO;
                }

                if score < singular_beta {
                    extension = 1;
                    extension += (!PV && score < singular_beta - 17) as i32;
                    extension += (!PV && is_quiet && score < singular_beta - 97) as i32;
                    if extension > 1 && depth < 12 {
                        depth += 1;
                    }
                } else if score >= beta {
                    return score;
                } else if tt_score >= beta {
                    extension = -2;
                } else if cut_node {
                    extension = -2;
                }
            }
        }

        let initial_nodes = td.counter.local();

        make_move(td, mv);

        let mut new_depth = depth + extension - 1;
        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 3 && move_count > 1 + is_root as i32 {
            reduction -= 90 * (history - 556) / 1024;
            reduction -= 3819 * correction_value.abs() / 1024;
            reduction -= 57 * move_count;
            reduction += 313;

            if tt_pv {
                reduction -= 792;
                reduction -= 597 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 717 * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if PV {
                reduction -= 622 + 619 * (beta - alpha > 32 * td.root_delta / 128) as i32;
            }

            if cut_node {
                reduction += 1141;
            }

            if td.board.in_check() {
                reduction -= 928;
            }

            if !improving {
                reduction += 749;
            }

            if td.stack[td.ply].cutoff_count > 2 {
                reduction += 770 + 62 * td.stack[td.ply].cutoff_count.max(7);
            }

            if td.stack[td.ply - 1].killer == mv {
                reduction -= 1043;
            }

            let reduced_depth = (new_depth - reduction / 1024)
                .clamp((PV && tt_move.is_some() && best_move.is_null()) as i32, new_depth + (PV || cut_node) as i32);

            td.stack[td.ply - 1].reduction = reduction;

            score = -search::<false>(td, -alpha - 1, -alpha, reduced_depth, true);

            td.stack[td.ply - 1].reduction = 0;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 46 + 542 * depth / 128) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);

                    if mv.is_quiet() {
                        let bonus = match score {
                            s if s >= beta => (1 + 2 * (move_count > depth) as i32) * (138 * depth - 54).min(1223),
                            s if s <= alpha => -(124 * depth - 60).min(1172),
                            _ => 0,
                        };

                        td.ply -= 1;
                        update_continuation_histories(td, td.stack[td.ply].piece, mv.to(), bonus);
                        td.ply += 1;
                    }
                }
            } else if score > alpha && score < best_score + 15 {
                new_depth -= 1;
            }
        }
        // Full Depth Search (FDS)
        else if !PV || move_count > 1 {
            td.stack[td.ply - 1].reduction = 1024 * ((depth - 1) - new_depth);
            score = -search::<false>(td, -alpha - 1, -alpha, new_depth, !cut_node);
            td.stack[td.ply - 1].reduction = 0;
        }

        // Principal Variation Search (PVS)
        if PV && (move_count == 1 || score > alpha) {
            score = -search::<true>(td, -beta, -alpha, new_depth, false);
        }

        undo_move(td, mv);

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

                if depth > 2 && depth < 16 && !is_decisive(score) {
                    depth -= 1;
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

    if best_move.is_some() {
        let bonus_noisy = (128 * depth - 60).min(1132);
        let malus_noisy = (141 * initial_depth - 75).min(1173) - 14 * (move_count - 1);

        let bonus_quiet = (129 * depth - 75).min(1370);
        let malus_quiet = (126 * initial_depth - 48).min(1245) - 18 * (move_count - 1);

        let bonus_cont = (115 * depth - 58).min(1357);
        let malus_cont = (204 * initial_depth - 60).min(911) - 15 * (move_count - 1);

        if best_move.is_noisy() {
            td.noisy_history.update(
                td.board.threats(),
                td.board.moved_piece(best_move),
                best_move.to(),
                td.board.piece_on(best_move.to()).piece_type(),
                bonus_noisy,
            );
        } else {
            td.stack[td.ply].killer = best_move;

            if !quiet_moves.is_empty() || depth > 3 {
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), best_move, bonus_quiet);
                update_continuation_histories(td, td.board.moved_piece(best_move), best_move.to(), bonus_cont);

                for &mv in quiet_moves.iter() {
                    td.quiet_history.update(td.board.threats(), td.board.side_to_move(), mv, -malus_quiet);
                    update_continuation_histories(td, td.board.moved_piece(mv), mv.to(), -malus_cont);
                }
            }
        }

        for &mv in noisy_moves.iter() {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.update(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured, -malus_noisy);
        }
    }

    if bound == Bound::Upper && td.ply >= 1 && (!quiet_moves.is_empty() || depth > 3) {
        tt_pv |= td.stack[td.ply - 1].tt_pv;

        let pcm_move = td.stack[td.ply - 1].mv;
        if pcm_move.is_some() && pcm_move.is_quiet() {
            let mut factor = 118;
            factor += 134 * (depth > 5) as i32;
            factor += 211 * (!in_check && best_score <= td.stack[td.ply].static_eval - 135) as i32;
            factor += 273
                * (is_valid(td.stack[td.ply - 1].static_eval) && best_score <= -td.stack[td.ply - 1].static_eval - 118)
                    as i32;

            let scaled_bonus = factor * (140 * depth - 51).min(1555) / 128;

            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);
        }
    }

    if PV {
        best_score = best_score.min(max_score);
    }

    if !excluded {
        td.tt.write(td.board.hash(), depth, raw_eval, best_score, bound, best_move, td.ply, tt_pv);
    }

    if !(in_check
        || best_move.is_noisy()
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

    if PV {
        td.pv.clear(td.ply);
    }

    td.counter.increment();

    if PV {
        td.sel_depth = td.sel_depth.max(td.ply as i32 + 1);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.board.is_draw(td.ply) {
        return Score::DRAW;
    }

    if td.ply >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { evaluate(td) };
    }

    let entry = td.tt.read(td.board.hash(), td.board.halfmove_clock(), td.ply);
    let mut tt_pv = PV;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    // QS Early TT-Cut
    if let Some(entry) = entry {
        tt_pv |= entry.pv;
        tt_score = entry.score;
        tt_bound = entry.bound;

        if is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha,
                Bound::Lower => tt_score >= beta,
                _ => true,
            }
        {
            debug_assert!(is_valid(tt_score));
            return tt_score;
        }
    }

    let mut best_score = -Score::INFINITE;
    let mut futility_score = Score::NONE;
    let mut raw_eval = Score::NONE;

    // Evaluation
    if !in_check {
        raw_eval = match entry {
            Some(entry) if is_valid(entry.eval) => entry.eval,
            _ => evaluate(td),
        };

        let static_eval = corrected_eval(raw_eval, correction_value(td), td.board.halfmove_clock());
        best_score = static_eval;

        if is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score < static_eval,
                Bound::Lower => tt_score > static_eval,
                _ => true,
            }
        {
            debug_assert!(is_valid(tt_score));
            best_score = tt_score;
        }

        if best_score >= beta {
            if entry.is_none() {
                td.tt.write(
                    td.board.hash(),
                    TtDepth::SOME,
                    raw_eval,
                    best_score,
                    Bound::Lower,
                    Move::NULL,
                    td.ply,
                    tt_pv,
                );
            }

            return best_score;
        }

        if best_score > alpha {
            alpha = best_score;
        }

        futility_score = static_eval + 131;
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

            if in_check && mv.is_quiet() {
                break;
            }

            if futility_score + PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] <= alpha {
                best_score = best_score.max(futility_score + PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()]);
                continue;
            }

            if futility_score <= alpha && !td.board.see(mv, 1) {
                best_score = best_score.max(futility_score);
                continue;
            }
        }

        make_move(td, mv);

        let score = -qsearch::<PV>(td, -beta, -alpha);

        undo_move(td, mv);

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = mv;
                alpha = score;

                if PV {
                    td.pv.update(td.ply, mv);
                }

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

    td.tt.write(td.board.hash(), TtDepth::SOME, raw_eval, best_score, bound, best_move, td.ply, tt_pv);

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn correction_value(td: &ThreadData) -> i32 {
    let stm = td.board.side_to_move();

    let correction = 1114 * td.pawn_corrhist.get(stm, td.board.pawn_key())
        + 975 * td.minor_corrhist.get(stm, td.board.minor_key())
        + 757 * td.major_corrhist.get(stm, td.board.major_key())
        + 1015 * td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + 1015 * td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black))
        + 992 * if td.ply >= 1 { td.last_move_corrhist.get(stm, td.stack[td.ply - 1].mv.encoded() as u64) } else { 0 };

    correction / 1024
}

fn corrected_eval(eval: i32, correction_value: i32, hmr: u8) -> i32 {
    (eval * (200 - hmr as i32) / 200 + correction_value).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX + 1)
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32) {
    let stm = td.board.side_to_move();

    td.pawn_corrhist.update(stm, td.board.pawn_key(), depth, diff);
    td.minor_corrhist.update(stm, td.board.minor_key(), depth, diff);
    td.major_corrhist.update(stm, td.board.major_key(), depth, diff);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), depth, diff);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), depth, diff);

    if td.ply >= 1 && td.stack[td.ply - 1].mv.is_some() {
        td.last_move_corrhist.update(td.board.side_to_move(), td.stack[td.ply - 1].mv.encoded() as u64, depth, diff);
    }
}

fn update_continuation_histories(td: &mut ThreadData, piece: Piece, sq: Square, bonus: i32) {
    if td.ply >= 1 {
        let entry = td.stack[td.ply - 1];
        if entry.mv.is_some() {
            td.continuation_history.update(entry.piece, entry.mv.to(), piece, sq, 1287 * bonus / 1024);
        }
    }

    if td.ply >= 2 {
        let entry = td.stack[td.ply - 2];
        if entry.mv.is_some() {
            td.continuation_history.update(entry.piece, entry.mv.to(), piece, sq, 1323 * bonus / 1024);
        }
    }

    if td.ply >= 3 {
        let entry = td.stack[td.ply - 3];
        if entry.mv.is_some() {
            td.continuation_history.update(entry.piece, entry.mv.to(), piece, sq, 937 * bonus / 1024);
        }
    }
}

fn make_move(td: &mut ThreadData, mv: Move) {
    td.stack[td.ply].piece = td.board.moved_piece(mv);
    td.stack[td.ply].mv = mv;
    td.ply += 1;

    td.nnue.push(mv, &td.board);
    td.board.make_move(mv);
    td.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.ply -= 1;
    td.nnue.pop();
    td.board.undo_move(mv);
}
