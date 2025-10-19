use crate::{
    evaluate::evaluate,
    movepick::{MovePicker, Stage},
    parameters::PIECE_VALUES,
    tb::{tb_probe, tb_rank_rootmoves, tb_size, GameOutcome},
    thread::{RootMove, ThreadData},
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

pub trait NodeType {
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

    td.pv_table.clear(0);
    td.nodes.clear_local();
    td.tb_hits.clear_local();

    td.nnue.full_refresh(&td.board);

    td.root_moves = td
        .board
        .generate_all_moves()
        .iter()
        .filter(|v| td.board.is_legal(v.mv))
        .map(|v| RootMove { mv: v.mv, ..Default::default() })
        .collect();

    let mut average = Score::NONE;
    let mut last_best_rootmove = RootMove::default();

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

        td.root_in_tb = false;
        td.stop_probing_tb = false;

        if td.board.castling().raw() == 0 && td.board.occupancies().len() <= tb_size() {
            tb_rank_rootmoves(td);
        }

        // Aspiration Windows
        if depth >= 2 {
            delta += average * average / 24616;

            alpha = (average - delta).max(-Score::INFINITE);
            beta = (average + delta).min(Score::INFINITE);

            td.optimism[td.board.side_to_move()] = 119 * average / (average.abs() + 237);
            td.optimism[!td.board.side_to_move()] = -td.optimism[td.board.side_to_move()];
        }

        loop {
            td.stack = Default::default();
            td.root_delta = beta - alpha;

            // Root Search
            let score = search::<Root>(td, alpha, beta, (depth - reduction).max(1), false, 0);

            td.nodes.flush();
            td.tb_hits.flush();

            td.root_moves.sort_by(|a, b| b.score.cmp(&a.score));

            if td.stopped {
                break;
            }

            match score {
                s if s <= alpha => {
                    beta = (alpha + beta) / 2;
                    alpha = (score - delta).max(-Score::INFINITE);
                    reduction = 0;
                    delta += delta * 30 / 128;
                }
                s if s >= beta => {
                    alpha = (beta - delta).max(alpha);
                    beta = (score + delta).min(Score::INFINITE);
                    reduction += 1;
                    delta += delta * (50 + 10 * reduction) / 128;
                }
                _ => {
                    average = if average == Score::NONE { score } else { (average + score) / 2 };
                    break;
                }
            }

            if report == Report::Full && td.nodes.global() > 10_000_000 {
                td.print_uci_info(depth);
            }
        }

        if !td.stopped {
            td.completed_depth = depth;
        }

        if report == Report::Full && !(is_loss(td.root_moves[0].display_score) && td.stopped) {
            td.print_uci_info(depth);
        }

        if last_best_rootmove.mv == td.root_moves[0].mv {
            pv_stability += 1;
        } else {
            pv_stability = 0;
        }

        if td.root_moves[0].score != -Score::INFINITE && is_loss(td.root_moves[0].score) && td.stopped {
            if let Some(pos) = td.root_moves.iter().position(|rm| rm.mv == last_best_rootmove.mv) {
                td.root_moves.remove(pos);
                td.root_moves.insert(0, last_best_rootmove.clone());
            }
        } else {
            last_best_rootmove = td.root_moves[0].clone();
        }

        if td.stopped {
            break;
        }

        if (td.root_moves[0].score - average).abs() < 12 {
            eval_stability += 1;
        } else {
            eval_stability = 0;
        }

        let multiplier = || {
            let nodes_factor = 2.15 - 1.5 * (td.root_moves[0].nodes as f32 / td.nodes.local() as f32);

            let pv_stability = 1.25 - 0.05 * pv_stability.min(8) as f32;

            let eval_stability = 1.2 - 0.04 * eval_stability.min(8) as f32;

            let score_trend = (0.8 + 0.05 * (td.previous_best_score - td.root_moves[0].score) as f32).clamp(0.80, 1.45);

            nodes_factor * pv_stability * eval_stability * score_trend
        };

        if td.time_manager.soft_limit(td, multiplier) {
            break;
        }
    }

    if report == Report::Minimal {
        td.print_uci_info(td.root_depth);
    }

    td.previous_best_score = td.root_moves[0].score;
}

fn search<NODE: NodeType>(
    td: &mut ThreadData, mut alpha: i32, mut beta: i32, depth: i32, cut_node: bool, ply: usize,
) -> i32 {
    debug_assert!(ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let in_check = td.board.in_check();
    let excluded = td.stack[ply].excluded.is_some();

    if !NODE::ROOT && NODE::PV {
        td.pv_table.clear(ply);
    }

    if td.stopped {
        return Score::ZERO;
    }

    // Qsearch Dive
    if depth <= 0 {
        return qsearch::<NODE>(td, alpha, beta, ply);
    }

    if !NODE::ROOT && alpha < Score::ZERO && td.board.upcoming_repetition(ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    if NODE::PV {
        td.sel_depth = td.sel_depth.max(ply as i32);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if !NODE::ROOT {
        if td.board.is_draw(ply) {
            return Score::DRAW;
        }

        if ply >= MAX_PLY - 1 {
            return if in_check { Score::DRAW } else { evaluate(td) };
        }

        // Mate Distance Pruning (MDP)
        alpha = alpha.max(mated_in(ply));
        beta = beta.min(mate_in(ply + 1));

        if alpha >= beta {
            return alpha;
        }
    }

    let mut best_score = -Score::INFINITE;
    let mut max_score = Score::INFINITE;

    let mut depth = depth.min(MAX_PLY as i32 - 1);
    let initial_depth = depth;

    let hash = td.board.hash();
    let (entry, tt_slot) = td.tt.read(hash, td.board.halfmove_clock(), ply);
    let mut tt_depth = 0;
    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    let mut tt_pv = NODE::PV;

    // Search Early TT-Cut
    if let Some(entry) = &entry {
        tt_move = entry.mv;
        tt_pv |= entry.pv;
        tt_score = entry.score;
        tt_depth = entry.depth;
        tt_bound = entry.bound;

        if !NODE::PV
            && !excluded
            && tt_depth > depth - (tt_score <= beta) as i32 - (tt_bound == Bound::Exact) as i32
            && is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha && (!cut_node || depth > 5),
                Bound::Lower => tt_score >= beta && (cut_node || depth > 5),
                _ => true,
            }
        {
            if tt_move.is_quiet() && tt_score >= beta {
                let quiet_bonus = (141 * depth - 72).min(1544) + 68 * !cut_node as i32;
                let conthist_bonus = (99 * depth - 61).min(1509) + 65 * !cut_node as i32;

                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), tt_move, quiet_bonus);
                update_continuation_histories(td, ply, td.board.moved_piece(tt_move), tt_move.to(), conthist_bonus);
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
        && !td.stop_probing_tb
    {
        if let Some(outcome) = tb_probe(&td.board) {
            td.tb_hits.increment();

            let (score, bound) = match outcome {
                GameOutcome::Win => (tb_win_in(ply), Bound::Lower),
                GameOutcome::Loss => (tb_loss_in(ply), Bound::Upper),
                GameOutcome::Draw => (Score::DRAW, Bound::Exact),
            };

            if bound == Bound::Exact
                || (bound == Bound::Lower && score >= beta)
                || (bound == Bound::Upper && score <= alpha)
            {
                let depth = (depth + 6).min(MAX_PLY as i32 - 1);
                td.tt.write(tt_slot, hash, depth, Score::NONE, score, bound, Move::NULL, ply, tt_pv);
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

    let correction_value = correction_value(td, ply);

    let raw_eval;
    let mut static_eval;
    let mut eval;

    // Evaluation
    if in_check {
        raw_eval = Score::NONE;
        static_eval = Score::NONE;
        eval = Score::NONE;

        if is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha,
                Bound::Lower => tt_score >= beta,
                _ => true,
            }
        {
            eval = tt_score;
            static_eval = tt_score;
        }
    } else if excluded {
        raw_eval = td.stack[ply].static_eval;
        static_eval = raw_eval;
        eval = static_eval;
    } else if let Some(entry) = &entry {
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
            eval = tt_score;
        }
    } else {
        raw_eval = evaluate(td);
        td.tt.write(tt_slot, hash, TtDepth::SOME, raw_eval, Score::NONE, Bound::None, Move::NULL, ply, tt_pv);

        static_eval = corrected_eval(raw_eval, correction_value, td.board.halfmove_clock());
        eval = static_eval;
    }

    td.stack[ply].static_eval = static_eval;
    td.stack[ply].tt_pv = tt_pv;
    td.stack[ply].reduction = 0;
    td.stack[ply].move_count = 0;
    td.stack[ply + 2].cutoff_count = 0;

    // Quiet Move Ordering Using Static-Eval
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[ply - 1].mv.is_quiet()
        && is_valid(td.stack[ply - 1].static_eval)
    {
        let value = 733 * (-(static_eval + td.stack[ply - 1].static_eval)) / 128;
        let bonus = value.clamp(-123, 255);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[ply - 1].mv, bonus);
    }

    // Hindsight reductions
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[ply - 1].reduction >= 2397
        && static_eval + td.stack[ply - 1].static_eval < 0
    {
        depth += 1;
    }

    if !NODE::ROOT
        && !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.stack[ply - 1].reduction >= 963
        && is_valid(td.stack[ply - 1].static_eval)
        && static_eval + td.stack[ply - 1].static_eval > 63
    {
        depth -= 1;
    }

    let potential_singularity =
        depth >= 5 && tt_depth >= depth - 3 && tt_bound != Bound::Upper && is_valid(tt_score) && !is_decisive(tt_score);

    let mut improvement = 0;

    if ply >= 2 && is_valid(td.stack[ply - 2].static_eval) && !in_check {
        improvement = static_eval - td.stack[ply - 2].static_eval;
    } else if ply >= 4 && is_valid(td.stack[ply - 4].static_eval) && !in_check {
        improvement = static_eval - td.stack[ply - 4].static_eval;
    }

    let improving = improvement > 0;

    // Razoring
    if !NODE::PV && !in_check && eval < alpha - 320 - 237 * initial_depth * initial_depth {
        return qsearch::<NonPV>(td, alpha, beta, ply);
    }

    // Static Evaluation Reverse Futility Pruning (SERFP)
    if !tt_pv
        && is_valid(eval)
        && !excluded
        && depth < 9
        && eval >= beta
        && static_eval >= beta + 75 * depth - (85 * improving as i32) + 580 * correction_value.abs() / 1024
        && !is_loss(beta)
        && !is_win(eval)
    {
        return beta + (static_eval - beta) / 3;
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && is_valid(eval)
        && !excluded
        && eval >= beta
        && eval
            >= beta + 157 * depth * depth / 16 + 31 * depth - (71 * improving as i32) - (23 * cut_node as i32)
                + 580 * correction_value.abs() / 1024
                + 24
        && !is_loss(beta)
        && !is_win(eval)
        && tt_bound != Bound::Upper
    {
        return (eval + beta) / 2;
    }

    // Null Move Pruning (NMP)
    if cut_node
        && !in_check
        && !excluded
        && eval >= beta
        && eval >= static_eval
        && static_eval >= beta - 16 * depth + 158 * tt_pv as i32 - 106 * improvement / 1024 + 213
        && ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
        && !potential_singularity
        && !is_loss(beta)
    {
        let r = (5756 + 321 * depth) / 1024;

        td.stack[ply].conthist = std::ptr::null_mut();
        td.stack[ply].contcorrhist = std::ptr::null_mut();
        td.stack[ply].piece = Piece::None;
        td.stack[ply].mv = Move::NULL;

        td.board.make_null_move();

        let score = -search::<NonPV>(td, -beta, -beta + 1, depth - r, false, ply + 1);

        td.board.undo_null_move();

        if td.stopped {
            return Score::ZERO;
        }

        if score >= beta && !is_win(score) {
            if td.nmp_min_ply > 0 || depth < 16 {
                return score;
            }

            td.nmp_min_ply = ply as i32 + 3 * (depth - r) / 4;
            let verified_score = search::<NonPV>(td, beta - 1, beta, depth - r, false, ply);
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
    let probcut_beta = beta + 259 - 65 * improving as i32;

    if cut_node
        && depth >= 3
        && !is_decisive(beta)
        && (!is_valid(tt_score) || tt_score >= probcut_beta && !is_decisive(tt_score))
        && !tt_move.is_quiet()
    {
        let mut move_picker = MovePicker::new_probcut(probcut_beta - static_eval);

        let probcut_depth = (depth - 4).max(0);

        let mut tried_tt_move = false;
        while let Some(mv) = move_picker.next::<NODE>(td, true, ply) {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if mv == td.stack[ply].excluded || !td.board.is_legal(mv) {
                continue;
            }

            tried_tt_move |= mv == tt_move;
            make_move(td, ply, mv);

            let mut score = -qsearch::<NonPV>(td, -probcut_beta, -probcut_beta + 1, ply + 1);

            if score >= probcut_beta && probcut_depth > 0 {
                score = -search::<NonPV>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, false, ply + 1);
            }

            undo_move(td, mv);

            if td.stopped {
                return Score::ZERO;
            }

            if score >= probcut_beta {
                td.tt.write(tt_slot, hash, probcut_depth + 1, raw_eval, score, Bound::Lower, mv, ply, tt_pv);

                if tt_move.is_some() && mv != tt_move && !tried_tt_move {
                    return score;
                }

                if !is_decisive(score) {
                    return score - (probcut_beta - beta);
                }
            }
        }
    }

    // Internal Iterative Reductions (IIR)
    if depth >= 3 + 3 * cut_node as i32 && tt_move.is_null() && (NODE::PV || cut_node) {
        depth -= 1;
    }

    let mut best_move = Move::NULL;
    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(tt_move);
    let mut skip_quiets = false;

    while let Some(mv) = move_picker.next::<NODE>(td, skip_quiets, ply) {
        if mv == td.stack[ply].excluded || !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;
        td.stack[ply].move_count = move_count;

        let is_quiet = mv.is_quiet();

        let history = if is_quiet {
            td.quiet_history.get(td.board.threats(), td.board.side_to_move(), mv)
                + td.conthist(ply, 1, mv)
                + td.conthist(ply, 2, mv)
        } else {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.get(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured)
        };

        let mut reduction = td.lmr.reduction(depth, move_count);

        if !improving {
            reduction += (489 - 412 * improvement / 128).min(1243);
        }

        if !NODE::ROOT && !is_loss(best_score) {
            let lmr_depth = (depth - reduction / 1024).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= !in_check
                && move_count
                    >= if improving || static_eval >= beta + 17 {
                        (3728 + 998 * initial_depth * initial_depth) / 1024
                    } else {
                        (1904 + 470 * initial_depth * initial_depth) / 1024
                    };

            // Futility Pruning (FP)
            let futility_value =
                static_eval + 105 * lmr_depth + 49 * history / 1024 + 95 * (static_eval >= alpha) as i32 + 83;

            if !in_check
                && is_quiet
                && lmr_depth < 9
                && futility_value <= alpha
                && !td.board.might_give_check_if_you_squint(mv)
            {
                if !is_decisive(best_score) && best_score <= futility_value {
                    best_score = futility_value;
                }
                skip_quiets = true;
                continue;
            }

            // Bad Noisy Futility Pruning (BNFP)
            let noisy_futility_value = static_eval
                + 123 * lmr_depth
                + 72 * history / 1024
                + 94 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 1024
                + 71;

            if !in_check && lmr_depth < 6 && move_picker.stage() == Stage::BadNoisy && noisy_futility_value <= alpha {
                if !is_decisive(best_score) && best_score <= noisy_futility_value {
                    best_score = noisy_futility_value;
                }
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet {
                -325 * lmr_depth * lmr_depth / 16 - 31 * history / 1024 + 16
            } else {
                -102 * depth - 45 * history / 1024 + 46
            };

            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        // Singular Extensions (SE)
        let mut extension = 0;

        if !NODE::ROOT && !excluded && ply < 2 * td.root_depth as usize && mv == tt_move && potential_singularity {
            debug_assert!(is_valid(tt_score));

            let singular_beta = tt_score - depth;
            let singular_depth = (depth - 1) / 2;

            td.stack[ply].excluded = tt_move;
            let score = search::<NonPV>(td, singular_beta - 1, singular_beta, singular_depth, cut_node, ply);
            td.stack[ply].excluded = Move::NULL;

            if td.stopped {
                return Score::ZERO;
            }

            if score < singular_beta {
                let double_margin = 2 + 277 * NODE::PV as i32;
                let triple_margin = 67 + 315 * NODE::PV as i32 - 16 * correction_value.abs() / 128;

                extension = 1;
                extension += (score < singular_beta - double_margin) as i32;
                extension += (score < singular_beta - triple_margin) as i32;

                if extension > 1 && depth < 14 {
                    depth += 1;
                }
            } else if score >= beta && !is_decisive(score) {
                return score;
            } else if tt_score >= beta {
                extension = -2;
            } else if cut_node {
                extension = -2;
            }
        }

        let initial_nodes = td.nodes.local();

        make_move(td, ply, mv);

        let mut new_depth = depth + extension - 1;
        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 2 && move_count > 1 {
            if is_quiet {
                reduction += 489;
                reduction -= 137 * history / 1024;
            } else {
                reduction += 488;
                reduction -= 109 * history / 1024;
                reduction -= 46 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128;
            }

            reduction -= 3607 * correction_value.abs() / 1024;
            reduction -= 69 * move_count;

            if tt_pv {
                reduction -= 427;
                reduction -= 677 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 729 * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if NODE::PV {
                reduction -= 393 + 552 * (beta - alpha) / td.root_delta;
            }

            if !tt_pv && cut_node {
                reduction += 1675;
                reduction += 934 * tt_move.is_null() as i32;
            }

            if td.board.in_check() || !td.board.has_non_pawns() {
                reduction -= 1049;
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += 1555;
            }

            if is_valid(tt_score) && tt_score < alpha && tt_bound == Bound::Upper {
                reduction += 791;
            }

            if depth == 2 {
                reduction -= 1397;
            }

            let reduced_depth = (new_depth - reduction / 1024).clamp(1, new_depth + 1) + 2 * NODE::PV as i32;

            td.stack[ply].reduction = reduction;
            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, true, ply + 1);
            td.stack[ply].reduction = 0;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 37 + 495 * depth / 128) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node, ply + 1);

                    if mv.is_quiet() && score >= beta {
                        let bonus = (1 + (move_count / depth)) * (155 * depth - 63).min(851);

                        update_continuation_histories(td, ply, td.stack[ply].piece, mv.to(), bonus);
                    }
                }
            } else if score > alpha && score < best_score + 16 {
                new_depth -= 1;
            }
        }
        // Full Depth Search (FDS)
        else if !NODE::PV || move_count > 1 {
            if is_quiet {
                reduction += 380;
                reduction -= 153 * history / 1024;
            } else {
                reduction += 355;
                reduction -= 68 * history / 1024;
                reduction -= 47 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128;
            }

            reduction -= 2667 * correction_value.abs() / 1024;
            reduction -= 52 * move_count;

            if tt_pv {
                reduction -= 750;
                reduction -= 537 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 1081 * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if NODE::PV {
                reduction -= 491 + 780 * (beta - alpha) / td.root_delta;
            }

            if !tt_pv && cut_node {
                reduction += 1478;
                reduction += 1048 * tt_move.is_null() as i32;
            }

            if td.board.in_check() || !td.board.has_non_pawns() {
                reduction -= 744;
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += 1438;
            }

            if is_valid(tt_score) && tt_score < alpha && tt_bound == Bound::Upper {
                reduction += 849;
            }

            if depth == 2 {
                reduction -= 1052;
            }

            if mv == tt_move {
                reduction -= 3034;
            }

            td.stack[ply].reduction = 1024 * ((initial_depth - 1) - new_depth);
            score =
                -search::<NonPV>(td, -alpha - 1, -alpha, new_depth - (reduction >= 3072) as i32, !cut_node, ply + 1);
            td.stack[ply].reduction = 0;
        }

        // Principal Variation Search (PVS)
        if NODE::PV && (move_count == 1 || score > alpha) {
            if mv == tt_move && tt_depth > 1 && td.root_depth > 8 {
                new_depth = new_depth.max(1);
            }

            score = -search::<PV>(td, -beta, -alpha, new_depth, false, ply + 1);
        }

        undo_move(td, mv);

        if td.stopped {
            return Score::ZERO;
        }

        if NODE::ROOT {
            let root_move = td.root_moves.iter_mut().find(|v| v.mv == mv).unwrap();

            root_move.nodes += td.nodes.local() - initial_nodes;

            if move_count == 1 || score > alpha {
                match score {
                    v if v <= alpha => {
                        root_move.display_score = alpha;
                        root_move.upperbound = true;
                    }
                    v if v >= beta => {
                        root_move.display_score = beta;
                        root_move.lowerbound = true;
                    }
                    _ => {
                        root_move.display_score = score;
                        root_move.upperbound = false;
                        root_move.lowerbound = false;
                    }
                }

                root_move.score = score;
                root_move.sel_depth = td.sel_depth;
                root_move.pv.commit_full_root_pv(&td.pv_table, 1);
            } else {
                root_move.score = -Score::INFINITE;
            }
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                bound = Bound::Exact;
                best_move = mv;

                if !NODE::ROOT && NODE::PV {
                    td.pv_table.update(ply, mv);
                }

                if score >= beta {
                    bound = Bound::Lower;
                    td.stack[ply].cutoff_count += 1;
                    break;
                }

                if depth > 2 && depth < 17 && !is_decisive(score) {
                    depth -= 1;
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
        if excluded {
            return alpha;
        }

        return if in_check { mated_in(ply) } else { Score::DRAW };
    }

    if best_move.is_some() {
        let bonus_noisy = (125 * depth - 57).min(1175) - 70 * cut_node as i32;
        let malus_noisy = (153 * initial_depth - 64).min(1476) - 24 * noisy_moves.len() as i32;

        let bonus_quiet = (152 * depth - 73).min(1569) - 64 * cut_node as i32;
        let malus_quiet = (133 * initial_depth - 51).min(1162) - 37 * quiet_moves.len() as i32;

        let bonus_cont = (102 * depth - 56).min(1223) - 65 * cut_node as i32;
        let malus_cont = (306 * initial_depth - 46).min(1018) - 30 * quiet_moves.len() as i32;

        if best_move.is_noisy() {
            td.noisy_history.update(
                td.board.threats(),
                td.board.moved_piece(best_move),
                best_move.to(),
                td.board.piece_on(best_move.to()).piece_type(),
                bonus_noisy,
            );
        } else {
            td.quiet_history.update(td.board.threats(), td.board.side_to_move(), best_move, bonus_quiet);
            update_continuation_histories(td, ply, td.board.moved_piece(best_move), best_move.to(), bonus_cont);

            for &mv in quiet_moves.iter() {
                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), mv, -malus_quiet);
                update_continuation_histories(td, ply, td.board.moved_piece(mv), mv.to(), -malus_cont);
            }
        }

        for &mv in noisy_moves.iter() {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.update(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured, -malus_noisy);
        }

        if !NODE::ROOT && td.stack[ply - 1].mv.is_quiet() && td.stack[ply - 1].move_count == 1 {
            let malus = (78 * initial_depth - 52).min(811);

            update_continuation_histories(td, ply - 1, td.stack[ply - 1].piece, td.stack[ply - 1].mv.to(), -malus);
        }
    }

    if !NODE::ROOT && bound == Bound::Upper {
        let pcm_move = td.stack[ply - 1].mv;
        if pcm_move.is_quiet() {
            let mut factor = 104;
            factor += 147 * (initial_depth > 5) as i32;
            factor += 217 * (!in_check && best_score <= static_eval.min(raw_eval) - 132) as i32;
            factor += 297
                * (is_valid(td.stack[ply - 1].static_eval) && best_score <= -td.stack[ply - 1].static_eval - 100)
                    as i32;

            let scaled_bonus = factor * (156 * initial_depth - 42).min(1789) / 128;

            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);

            if ply >= 2 {
                let entry = &td.stack[ply - 2];
                if entry.mv.is_some() {
                    let bonus = (151 * initial_depth - 41).min(1630);
                    td.continuation_history.update(entry.conthist, td.stack[ply - 1].piece, pcm_move.to(), bonus);
                }
            }
        }
    }

    tt_pv |= !NODE::ROOT && bound == Bound::Upper && td.stack[ply - 1].tt_pv && (!quiet_moves.is_empty() || depth > 3);

    debug_assert!(alpha < beta);
    if best_score >= beta && !is_decisive(best_score) && !is_decisive(alpha) {
        best_score = (best_score * depth + beta) / (depth + 1);
    }

    if NODE::PV {
        best_score = best_score.min(max_score);
    }

    if !excluded {
        td.tt.write(tt_slot, hash, depth, raw_eval, best_score, bound, best_move, ply, tt_pv);
    }

    if !(in_check
        || best_move.is_noisy()
        || (bound == Bound::Upper && best_score >= static_eval)
        || (bound == Bound::Lower && best_score <= static_eval))
    {
        update_correction_histories(td, depth, best_score - static_eval, ply);
    }

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn qsearch<NODE: NodeType>(td: &mut ThreadData, mut alpha: i32, beta: i32, ply: usize) -> i32 {
    debug_assert!(!NODE::ROOT);
    debug_assert!(ply <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    if alpha < Score::ZERO && td.board.upcoming_repetition(ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    let in_check = td.board.in_check();

    if NODE::PV {
        td.pv_table.clear(ply);
        td.sel_depth = td.sel_depth.max(ply as i32);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.board.is_draw(ply) {
        return Score::DRAW;
    }

    if ply >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { evaluate(td) };
    }

    let hash = td.board.hash();
    let (entry, tt_slot) = td.tt.read(hash, td.board.halfmove_clock(), ply);
    let mut tt_pv = NODE::PV;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    // QS Early TT-Cut
    if let Some(entry) = &entry {
        tt_pv |= entry.pv;
        tt_score = entry.score;
        tt_bound = entry.bound;

        if is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha,
                Bound::Lower => tt_score >= beta,
                _ => true,
            }
            && (!NODE::PV || !is_decisive(tt_score))
        {
            return tt_score;
        }
    }

    let mut best_score = -Score::INFINITE;
    let mut futility_base = Score::NONE;
    let mut raw_eval = Score::NONE;

    // Evaluation
    if !in_check {
        raw_eval = match &entry {
            Some(entry) if is_valid(entry.eval) => entry.eval,
            _ => evaluate(td),
        };
        best_score = corrected_eval(raw_eval, correction_value(td, ply), td.board.halfmove_clock());

        if is_valid(tt_score)
            && (!NODE::PV || !is_decisive(tt_score))
            && match tt_bound {
                Bound::Upper => tt_score < best_score,
                Bound::Lower => tt_score > best_score,
                _ => true,
            }
        {
            best_score = tt_score;
        }

        if best_score >= beta {
            if !is_decisive(best_score) && !is_decisive(beta) {
                best_score = (best_score + beta) / 2;
            }

            if entry.is_none() {
                td.tt.write(tt_slot, hash, TtDepth::SOME, raw_eval, best_score, Bound::Lower, Move::NULL, ply, tt_pv);
            }

            return best_score;
        }

        if best_score > alpha {
            alpha = best_score;
        }

        futility_base = best_score + 79;
    }

    let mut best_move = Move::NULL;

    let mut move_count = 0;
    let mut move_picker = MovePicker::new_qsearch();

    let previous_square = match td.stack[ply - 1].mv {
        Move::NULL => Square::None,
        _ => td.stack[ply - 1].mv.to(),
    };

    while let Some(mv) = move_picker.next::<NODE>(td, !in_check, ply) {
        if !td.board.is_legal(mv) {
            continue;
        }

        if NODE::ROOT && td.root_in_tb {
            debug_assert!(td.root_moves[0].tb_rank == td.root_moves.iter().map(|rm| rm.tb_rank).max().unwrap_or(0));

            if td.root_moves.iter().any(|rm| rm.mv == mv && rm.tb_rank != td.root_moves[0].tb_rank) {
                continue;
            }
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

            let futility_score = futility_base + 32 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128;

            if !in_check && futility_score <= alpha && !td.board.see(mv, 1) {
                best_score = best_score.max(futility_score);
                continue;
            }
        }

        if !is_loss(best_score) && !td.board.see(mv, -79) {
            continue;
        }

        make_move(td, ply, mv);

        let score = -qsearch::<NODE>(td, -beta, -alpha, ply + 1);

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
                    td.pv_table.update(ply, mv);
                }

                if score >= beta {
                    break;
                }
            }
        }
    }

    if in_check && move_count == 0 {
        return mated_in(ply);
    }

    if best_score >= beta && !is_decisive(best_score) && !is_decisive(beta) {
        best_score = (best_score + beta) / 2;
    }

    let bound = if best_score >= beta { Bound::Lower } else { Bound::Upper };

    td.tt.write(tt_slot, hash, TtDepth::SOME, raw_eval, best_score, bound, best_move, ply, tt_pv);

    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn correction_value(td: &ThreadData, ply: usize) -> i32 {
    let stm = td.board.side_to_move();

    let mut correction = td.pawn_corrhist.get(stm, td.board.pawn_key())
        + td.minor_corrhist.get(stm, td.board.minor_key())
        + td.major_corrhist.get(stm, td.board.major_key())
        + td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black));

    if ply >= 2 && td.stack[ply - 1].mv.is_some() && td.stack[ply - 2].mv.is_some() {
        correction += td.continuation_corrhist.get(
            td.stack[ply - 2].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
        );
    }

    if ply >= 4 && td.stack[ply - 1].mv.is_some() && td.stack[ply - 4].mv.is_some() {
        correction += td.continuation_corrhist.get(
            td.stack[ply - 4].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
        );
    }

    correction
}

fn corrected_eval(eval: i32, correction_value: i32, hmr: u8) -> i32 {
    (eval * (200 - hmr as i32) / 200 + correction_value).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX - 1)
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32, ply: usize) {
    let stm = td.board.side_to_move();
    let bonus = (150 * depth * diff / 128).clamp(-4194, 3164);

    td.pawn_corrhist.update(stm, td.board.pawn_key(), bonus);
    td.minor_corrhist.update(stm, td.board.minor_key(), bonus);
    td.major_corrhist.update(stm, td.board.major_key(), bonus);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), bonus);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), bonus);

    if ply >= 2 && td.stack[ply - 1].mv.is_some() && td.stack[ply - 2].mv.is_some() {
        td.continuation_corrhist.update(
            td.stack[ply - 2].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
            bonus,
        );
    }

    if ply >= 4 && td.stack[ply - 1].mv.is_some() && td.stack[ply - 4].mv.is_some() {
        td.continuation_corrhist.update(
            td.stack[ply - 4].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
            bonus,
        );
    }
}

fn update_continuation_histories(td: &mut ThreadData, ply: usize, piece: Piece, sq: Square, bonus: i32) {
    for offset in [1, 2, 3, 4, 6] {
        if ply >= offset {
            let entry = &td.stack[ply - offset];
            if entry.mv.is_some() {
                td.continuation_history.update(entry.conthist, piece, sq, bonus);
            }
        }
    }
}

fn make_move(td: &mut ThreadData, ply: usize, mv: Move) {
    td.stack[ply].mv = mv;
    td.stack[ply].piece = td.board.moved_piece(mv);
    td.stack[ply].conthist =
        td.continuation_history.subtable_ptr(td.board.in_check(), mv.is_noisy(), td.board.moved_piece(mv), mv.to());
    td.stack[ply].contcorrhist =
        td.continuation_corrhist.subtable_ptr(td.board.in_check(), mv.is_noisy(), td.board.moved_piece(mv), mv.to());

    td.nodes.increment();
    td.nnue.push(mv, &td.board);
    td.board.make_move(mv);
    td.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.nnue.pop();
    td.board.undo_move(mv);
}
