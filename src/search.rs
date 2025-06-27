use std::time::Instant;

use crate::{
    evaluate::evaluate,
    movepick::{MovePicker, Stage},
    parameters::PIECE_VALUES,
    tb::{tb_probe, tb_size, GameOutcome},
    thread::ThreadData,
    transposition::{Bound, TtDepth},
    types::{
        is_decisive, is_loss, is_valid, is_win, mate_in, mated_in, tb_loss_in, tb_win_in, ArrayVec, Color, Move, Piece,
        Score, Square, MAX_PLY,
    },
};

#[allow(unused_imports)]
use crate::misc::{dbg_hit, dbg_stats};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Report {
    None,
    Minimal,
    Full,
}

trait NodeType {
    const PV: bool;
    const ROOT: bool;
}

struct Root;
impl NodeType for Root {
    const PV: bool = true;
    const ROOT: bool = true;
}

struct PV;
impl NodeType for PV {
    const PV: bool = true;
    const ROOT: bool = false;
}

struct NonPV;
impl NodeType for NonPV {
    const PV: bool = false;
    const ROOT: bool = false;
}

pub fn start(td: &mut ThreadData, report: Report) {
    td.completed_depth = 0;
    td.stopped = false;

    td.pv.clear(0);
    td.node_table.clear();
    td.counter.clear();
    td.tb_hits.clear();

    td.nnue.full_refresh(&td.board);

    let now = Instant::now();

    let mut average = Score::NONE;
    let mut last_move = Move::NULL;

    let mut eval_stability = 0;
    let mut pv_stability = 0;

    // Iterative Deepening
    for depth in 1..MAX_PLY as i32 {
        td.sel_depth = 0;
        td.root_depth = depth;

        let mut alpha = -Score::INFINITE;
        let mut beta = Score::INFINITE;

        let mut delta = 12;
        let mut reduction = 0;

        // Aspiration Windows
        if depth >= 4 {
            delta += average * average / 26411;

            alpha = (average - delta).max(-Score::INFINITE);
            beta = (average + delta).min(Score::INFINITE);

            td.optimism[td.board.side_to_move()] = 114 * average / (average.abs() + 240);
            td.optimism[!td.board.side_to_move()] = -td.optimism[td.board.side_to_move()];
        }

        loop {
            td.stack = Default::default();
            td.root_delta = beta - alpha;

            // Root Search
            let score = search::<Root>(td, alpha, beta, (depth - reduction).max(1), false);

            if td.stopped {
                break;
            }

            match score {
                s if s <= alpha => {
                    beta = (alpha + beta) / 2;
                    alpha = (score - delta).max(-Score::INFINITE);
                    reduction = 0;
                }
                s if s >= beta => {
                    beta = (score + delta).min(Score::INFINITE);
                    reduction += 1;
                }
                _ => {
                    average = if average == Score::NONE { score } else { (average + score) / 2 };
                    break;
                }
            }

            delta += delta * (44 + 14 * reduction) / 128;
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

        let multiplier = || {
            let nodes_factor = 2.15 - 1.5 * (td.node_table.get(td.pv.best_move()) as f32 / td.counter.local() as f32);

            let pv_stability = 1.25 - 0.05 * pv_stability as f32;

            let eval_stability = 1.2 - 0.04 * eval_stability as f32;

            let score_trend = (800 + 20 * (td.previous_best_score - td.best_score)).clamp(750, 1500) as f32 / 1000.0;

            nodes_factor * pv_stability * eval_stability * score_trend
        };

        if td.time_manager.soft_limit(td, multiplier) {
            break;
        }

        if report == Report::Full {
            td.print_uci_info(depth, td.best_score, now);
        }
    }

    if report != Report::None {
        td.print_uci_info(td.root_depth, td.best_score, now);
    }

    td.previous_best_score = td.best_score;
}

fn search<NODE: NodeType>(td: &mut ThreadData, mut alpha: i32, mut beta: i32, depth: i32, cut_node: bool) -> i32 {
    debug_assert!(td.ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let in_check = td.board.in_check();
    let excluded = td.stack[td.ply].excluded.is_some();

    if NODE::PV {
        td.pv.clear(td.ply);
    }

    if td.stopped {
        return Score::ZERO;
    }

    if !NODE::ROOT && alpha < Score::ZERO && td.board.upcoming_repetition(td.ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    // Qsearch Dive
    if depth <= 0 {
        return qsearch::<NODE>(td, alpha, beta);
    }

    if NODE::PV {
        td.sel_depth = td.sel_depth.max(td.ply as i32 + 1);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if !NODE::ROOT {
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
    let initial_depth = depth;

    let entry = &td.tt.read(td.board.hash(), td.board.halfmove_clock(), td.ply);
    let mut tt_depth = 0;
    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    let mut tt_pv = NODE::PV;

    // Search Early TT-Cut
    if let Some(entry) = entry {
        tt_move = entry.mv;
        tt_pv |= entry.pv;
        tt_score = entry.score;
        tt_depth = entry.depth;
        tt_bound = entry.bound;

        if !NODE::PV
            && !excluded
            && tt_depth >= depth
            && is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha && (!cut_node || depth > 5),
                Bound::Lower => tt_score >= beta && (cut_node || depth > 5),
                _ => true,
            }
        {
            if tt_move.is_quiet() && tt_score >= beta {
                let quiet_bonus = (137 * depth - 73).min(1405);
                let conthist_bonus = (105 * depth - 63).min(1435);

                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), tt_move, quiet_bonus);
                update_continuation_histories(td, td.board.moved_piece(tt_move), tt_move.to(), conthist_bonus);
            }

            if td.board.halfmove_clock() < 90 {
                debug_assert!(is_valid(tt_score));
                return tt_score;
            }
        }
    }

    // Tablebases Probe
    if !NODE::ROOT
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

            if NODE::PV {
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
    td.stack[td.ply + 2].cutoff_count = 0;

    // Quiet Move Ordering Using Static-Eval
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[td.ply - 1].mv.is_quiet()
        && is_valid(td.stack[td.ply - 1].static_eval)
    {
        let value = 674 * (-(static_eval + td.stack[td.ply - 1].static_eval)) / 128;
        let bonus = value.clamp(-61, 144);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[td.ply - 1].mv, bonus);
    }

    // Hindsight LMR
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[td.ply - 1].reduction >= 2691
        && static_eval + td.stack[td.ply - 1].static_eval < 0
    {
        depth += 1;
    }

    if !NODE::ROOT
        && !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.stack[td.ply - 1].reduction >= 905
        && is_valid(td.stack[td.ply - 1].static_eval)
        && static_eval + td.stack[td.ply - 1].static_eval > 69
    {
        depth -= 1;
    }

    let potential_singularity =
        depth >= 5 && tt_depth >= depth - 3 && tt_bound != Bound::Upper && is_valid(tt_score) && !is_decisive(tt_score);

    let improving =
        !in_check && td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && static_eval > td.stack[td.ply - 2].static_eval;

    // Razoring
    if !NODE::PV && !in_check && eval < alpha - 303 - 260 * depth * depth {
        return qsearch::<NonPV>(td, alpha, beta);
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && !in_check
        && !excluded
        && depth <= 7
        && eval >= beta
        && eval
            >= beta + 80 * depth - (72 * improving as i32) - (25 * cut_node as i32)
                + 556 * correction_value.abs() / 1024
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
        && static_eval >= beta - 15 * depth + 159 * tt_pv as i32 + 203
        && td.ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
        && !potential_singularity
    {
        let r = 5 + depth / 3 + ((eval - beta) / 225).min(3);

        td.stack[td.ply].conthist = std::ptr::null_mut();
        td.stack[td.ply].contcorrhist = std::ptr::null_mut();
        td.stack[td.ply].piece = Piece::None;
        td.stack[td.ply].mv = Move::NULL;
        td.ply += 1;

        td.board.make_null_move();

        td.stack[td.ply].reduction = 1024 * (r - 1);
        let mut score = -search::<NonPV>(td, -beta, -beta + 1, depth - r, false);
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
            let verified_score = search::<NonPV>(td, beta - 1, beta, depth - r, false);
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
    let probcut_beta = beta + 280 - 63 * improving as i32;

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

            let mut score = -qsearch::<NonPV>(td, -probcut_beta, -probcut_beta + 1);

            if score >= probcut_beta && probcut_depth > 0 {
                td.stack[td.ply].reduction = 1024 * (initial_depth - 1 - probcut_depth);
                score = -search::<NonPV>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, !cut_node);
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
    if depth >= 3 + 3 * cut_node as i32 && tt_move.is_null() && (NODE::PV || cut_node) {
        depth -= 1;
    }

    let mut best_move = Move::NULL;
    let mut had_best_noisy_move = false;

    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(tt_move);
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

        if !improving {
            reduction += 800;
        }

        if !NODE::ROOT && !is_loss(best_score) {
            let lmr_depth = (depth - reduction / 1024 + is_quiet as i32 * history / 7657).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= move_count >= (4 + depth * depth) / (2 - (improving || static_eval >= beta + 18) as i32);

            // Futility Pruning (FP)
            let futility_value = static_eval + 122 * lmr_depth + 78;
            if !in_check && is_quiet && lmr_depth < 9 && futility_value <= alpha {
                if !is_decisive(best_score) && best_score <= futility_value {
                    best_score = futility_value;
                }
                skip_quiets = true;
                continue;
            }

            // Bad Noisy Futility Pruning (BNFP)
            let capt_futility_value = static_eval
                + 111 * lmr_depth
                + 396 * move_count / 128
                + PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 12
                + 80 * (history + 500) / 1024;

            if !in_check && lmr_depth < 6 && move_picker.stage() == Stage::BadNoisy && capt_futility_value <= alpha {
                if !is_decisive(best_score) && best_score <= capt_futility_value {
                    best_score = capt_futility_value;
                }
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet {
                -24 * lmr_depth * lmr_depth - 43 * history / 1024
            } else {
                -94 * depth + 48 - 42 * history / 1024
            };

            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        // Singular Extensions (SE)
        let mut extension = 0;

        if !NODE::ROOT && !excluded && td.ply < 2 * td.root_depth as usize && mv == tt_move {
            let entry = &entry.unwrap();

            if potential_singularity {
                debug_assert!(is_valid(tt_score));
                let singular_beta = tt_score - depth;
                let singular_depth = (depth - 1) / 2;

                td.stack[td.ply].excluded = entry.mv;
                let score = search::<NonPV>(td, singular_beta - 1, singular_beta, singular_depth, cut_node);
                td.stack[td.ply].excluded = Move::NULL;

                if td.stopped {
                    return Score::ZERO;
                }

                if score < singular_beta {
                    extension = 1;
                    extension += (!NODE::PV && score < singular_beta - 2) as i32;
                    extension += (!NODE::PV && is_quiet && score < singular_beta - 64) as i32;
                    if extension > 1 && depth < 14 {
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
        if depth >= 3 && move_count > 1 + NODE::ROOT as i32 {
            reduction -= 98 * (history - 568) / 1024;
            reduction -= 3295 * correction_value.abs() / 1024;
            reduction -= 54 * move_count;
            reduction += 295;

            if tt_pv {
                reduction -= 683;
                reduction -= 647 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 791 * (is_valid(tt_score) && tt_depth >= depth) as i32;
                reduction -= 768 * cut_node as i32;
            }

            if NODE::PV {
                reduction -= 614 + 576 * (beta - alpha > 34 * td.root_delta / 128) as i32;
            }

            if cut_node {
                reduction += 1141;
            }

            if td.board.in_check() {
                reduction -= 820;
            }

            if td.stack[td.ply].cutoff_count > 2 {
                reduction += 1196;
            }

            let reduced_depth =
                (new_depth - reduction / 1024).clamp(NODE::PV as i32, new_depth + (NODE::PV || cut_node) as i32);

            td.stack[td.ply - 1].reduction = reduction;

            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, true);

            td.stack[td.ply - 1].reduction = 0;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 46 + 512 * depth / 128) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    td.stack[td.ply - 1].reduction = 1024 * ((initial_depth - 1) - new_depth);
                    score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node);
                    td.stack[td.ply - 1].reduction = 0;

                    if mv.is_quiet() && score >= beta {
                        let bonus = (1 + 2 * (move_count > depth) as i32 + 2 * (move_count > 2 * depth) as i32)
                            * (152 * depth - 50).min(973);
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
        else if !NODE::PV || move_count > 1 {
            td.stack[td.ply - 1].reduction = 1024 * ((initial_depth - 1) - new_depth);
            score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node);
            td.stack[td.ply - 1].reduction = 0;
        }

        // Principal Variation Search (PVS)
        if NODE::PV && (move_count == 1 || score > alpha) {
            score = -search::<PV>(td, -beta, -alpha, new_depth, false);
        }

        undo_move(td, mv);

        if td.stopped {
            return Score::ZERO;
        }

        if NODE::ROOT {
            td.node_table.add(mv, td.counter.local() - initial_nodes);
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                bound = Bound::Exact;
                alpha = score;
                best_move = mv;

                if best_move.is_noisy() {
                    had_best_noisy_move = true;
                }
                if NODE::PV {
                    td.pv.update(td.ply, mv);

                    if NODE::ROOT {
                        td.best_score = score;
                    }
                }

                if score >= beta {
                    bound = Bound::Lower;
                    td.stack[td.ply].cutoff_count += 1;
                    break;
                }

                if depth > 2 && depth < 15 && !is_decisive(score) {
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
        let bonus_noisy = (124 * depth - 65).min(1177);
        let malus_noisy = (145 * initial_depth - 75).min(1403) - 14 * (move_count - 1);

        let bonus_quiet = (148 * depth - 71).min(1458);
        let malus_quiet = (125 * initial_depth - 52).min(1263) - 17 * (move_count - 1) + 196 * skip_quiets as i32;

        let bonus_cont = (114 * depth - 53).min(1318);
        let malus_cont = (244 * initial_depth - 51).min(907) - 15 * (move_count - 1) + 128 * skip_quiets as i32;

        if best_move.is_noisy() {
            td.noisy_history.update(
                td.board.threats(),
                td.board.moved_piece(best_move),
                best_move.to(),
                td.board.piece_on(best_move.to()).piece_type(),
                bonus_noisy,
            );
        } else if !quiet_moves.is_empty() || depth > 3 {
            td.quiet_history.update(td.board.threats(), td.board.side_to_move(), best_move, bonus_quiet);
            update_continuation_histories(td, td.board.moved_piece(best_move), best_move.to(), bonus_cont);

            for &mv in quiet_moves.iter() {
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), mv, -malus_quiet);
                update_continuation_histories(td, td.board.moved_piece(mv), mv.to(), -malus_cont);
            }
        }

        for &mv in noisy_moves.iter() {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.update(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured, -malus_noisy);
        }
    }

    if !NODE::ROOT && bound == Bound::Upper && (!quiet_moves.is_empty() || depth > 3) {
        tt_pv |= td.stack[td.ply - 1].tt_pv;

        let pcm_move = td.stack[td.ply - 1].mv;
        if pcm_move.is_quiet() {
            let mut factor = 102;
            factor += 141 * (depth > 5) as i32;
            factor += 227 * (!in_check && best_score <= static_eval - 129) as i32;
            factor += 277
                * (is_valid(td.stack[td.ply - 1].static_eval) && best_score <= -td.stack[td.ply - 1].static_eval - 101)
                    as i32;

            let scaled_bonus = factor * (137 * depth - 43).min(1563) / 128;

            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);
        }
    }

    if NODE::PV {
        best_score = best_score.min(max_score);
    }

    if !excluded {
        td.tt.write(td.board.hash(), depth, raw_eval, best_score, bound, best_move, td.ply, tt_pv);
    }

    if !(in_check
        || had_best_noisy_move
        || (bound == Bound::Upper && best_score >= static_eval)
        || (bound == Bound::Lower && best_score <= static_eval))
    {
        update_correction_histories(td, depth, best_score - static_eval);
    }

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn qsearch<NODE: NodeType>(td: &mut ThreadData, mut alpha: i32, beta: i32) -> i32 {
    debug_assert!(td.ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let in_check = td.board.in_check();

    if NODE::PV {
        td.pv.clear(td.ply);
    }

    if NODE::PV {
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

    let entry = &td.tt.read(td.board.hash(), td.board.halfmove_clock(), td.ply);
    let mut tt_pv = NODE::PV;
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
            if !is_decisive(best_score) && !is_decisive(beta) {
                best_score = (best_score + beta) / 2;
            }

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

        futility_score = static_eval + 129;
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

            if !in_check && futility_score <= alpha && !td.board.see(mv, 1) {
                best_score = best_score.max(futility_score);
                continue;
            }

            if !td.board.see(mv, -75) {
                continue;
            }
        }

        make_move(td, mv);

        let score = -qsearch::<NODE>(td, -beta, -alpha);

        undo_move(td, mv);

        if td.stopped {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = mv;
                alpha = score;

                if NODE::PV {
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

    let mut correction = 1074 * td.pawn_corrhist.get(stm, td.board.pawn_key())
        + 919 * td.minor_corrhist.get(stm, td.board.minor_key())
        + 724 * td.major_corrhist.get(stm, td.board.major_key())
        + 1058 * td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + 1058 * td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black));

    if td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && td.stack[td.ply - 2].mv.is_some() {
        correction += 1024
            * td.continuation_corrhist.get(
                td.stack[td.ply - 2].contcorrhist,
                td.stack[td.ply - 1].piece,
                td.stack[td.ply - 1].mv.to(),
            );
    }

    correction / 1024
}

fn corrected_eval(eval: i32, correction_value: i32, hmr: u8) -> i32 {
    (eval * (200 - hmr as i32) / 200 + correction_value).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX + 1)
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32) {
    let stm = td.board.side_to_move();
    let bonus = (depth * diff).clamp(-3927, 3373);

    td.pawn_corrhist.update(stm, td.board.pawn_key(), 1026 * bonus / 1024);
    td.minor_corrhist.update(stm, td.board.minor_key(), 1159 * bonus / 1024);
    td.major_corrhist.update(stm, td.board.major_key(), 929 * bonus / 1024);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), 1129 * bonus / 1024);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), 1056 * bonus / 1024);

    if td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && td.stack[td.ply - 2].mv.is_some() {
        td.continuation_corrhist.update(
            td.stack[td.ply - 2].contcorrhist,
            td.stack[td.ply - 1].piece,
            td.stack[td.ply - 1].mv.to(),
            1024 * bonus / 1024,
        );
    }
}

fn update_continuation_histories(td: &mut ThreadData, piece: Piece, sq: Square, bonus: i32) {
    const BONUSES: [(usize, i32); 5] = [(1, 1523), (2, 1144), (3, 957), (4, 1024), (6, 1024)];

    for (offset, scale) in BONUSES {
        if td.ply >= offset {
            let entry = &td.stack[td.ply - offset];
            if entry.mv.is_some() {
                td.continuation_history.update(entry.conthist, piece, sq, scale * bonus / 1024);
            }
        }
    }
}

fn make_move(td: &mut ThreadData, mv: Move) {
    td.stack[td.ply].conthist = td.continuation_history.subtable_ptr(td.board.moved_piece(mv), mv.to());
    td.stack[td.ply].contcorrhist = td.continuation_corrhist.subtable_ptr(td.board.moved_piece(mv), mv.to());
    td.stack[td.ply].piece = td.board.moved_piece(mv);
    td.stack[td.ply].mv = mv;
    td.ply += 1;

    td.counter.increment();
    td.nnue.push(mv, &td.board);
    td.board.make_move(mv);
    td.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.ply -= 1;
    td.nnue.pop();
    td.board.undo_move(mv);
}
