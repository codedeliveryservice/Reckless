use std::sync::atomic::Ordering;

use crate::{
    evaluation::correct_eval,
    movepick::{MovePicker, Stage},
    parameters::*,
    stack::Stack,
    thread::{RootMove, Status, ThreadData},
    time::Limits,
    transposition::{Bound, TtDepth},
    types::{
        ArrayVec, Color, MAX_PLY, Move, Piece, PieceType, Score, Square, draw, is_decisive, is_loss, is_valid, is_win,
        mate_in, mated_in,
    },
};

#[cfg(feature = "syzygy")]
use crate::{
    tb,
    types::{tb_loss_in, tb_win_in},
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

pub fn start(td: &mut ThreadData, report: Report, thread_count: usize) {
    td.completed_depth = 0;

    td.pv_table.clear(0);
    td.nnue.full_refresh(&td.board);

    td.root_moves = td.board.generate_all_moves().iter().map(|v| RootMove { mv: v.mv, ..Default::default() }).collect();

    td.root_in_tb = false;
    td.stop_probing_tb = false;

    #[cfg(feature = "syzygy")]
    if td.board.castling().raw() == 0 && td.board.occupancies().popcount() <= tb::size() {
        tb::rank_rootmoves(td);
    }

    td.multi_pv = td.multi_pv.min(td.root_moves.len());

    let mut average = vec![td.previous_best_score; td.multi_pv];
    let mut last_best_rootmove = RootMove::default();

    let mut eval_stability = 0;
    let mut pv_stability = 0;
    let mut best_move_changes = 0;
    let mut soft_stop_voted = false;

    // Iterative Deepening
    for depth in 1..MAX_PLY as i32 {
        if td.id == 0
            && let Limits::Depth(maximum) = td.time_manager.limits()
            && depth > maximum
        {
            td.shared.status.set(Status::STOPPED);
            break;
        }
        best_move_changes /= 2;

        td.sel_depth = 0;
        td.root_depth = depth;
        td.best_move_changes = 0;

        td.pv_start = 0;
        td.pv_end = 0;

        for rm in &mut td.root_moves {
            rm.previous_score = rm.score;
        }

        let mut delta = delta1();
        let mut reduction = 0;

        for index in 0..td.multi_pv {
            td.pv_index = index;

            if td.pv_index == td.pv_end {
                td.pv_start = td.pv_end;
                while td.pv_end < td.root_moves.len() {
                    if td.root_moves[td.pv_end].tb_rank != td.root_moves[td.pv_start].tb_rank {
                        break;
                    }
                    td.pv_end += 1;
                }
            }

            // Aspiration Windows
            delta += average[td.pv_index] * average[td.pv_index] / delta2();

            let mut alpha = (average[td.pv_index] - delta).max(-Score::INFINITE);
            let mut beta = (average[td.pv_index] + delta).min(Score::INFINITE);

            let best_avg = ((td.shared.best_stats[td.pv_index].load(Ordering::Acquire) & 0xffff) as i32 - 32768
                + average[td.pv_index])
                / 2;
            td.optimism[td.board.side_to_move()] = opt1() * best_avg / (best_avg.abs() + opt2());
            td.optimism[!td.board.side_to_move()] = -td.optimism[td.board.side_to_move()];

            loop {
                td.stack = Stack::default();
                td.root_delta = beta - alpha;

                // Root Search
                let score = search::<Root>(td, alpha, beta, (depth - reduction).max(1), false, 0);

                td.root_moves[td.pv_index..td.pv_end].sort_by_key(|rm| std::cmp::Reverse(rm.score));

                if td.shared.status.get() == Status::STOPPED {
                    break;
                }

                match score {
                    s if s <= alpha => {
                        beta = (3 * alpha + beta) / 4;
                        alpha = (score - delta).max(-Score::INFINITE);
                        reduction = 0;
                        delta += delta3() * delta / 128;
                    }
                    s if s >= beta => {
                        alpha = (beta - delta).max(alpha);
                        beta = (score + delta).min(Score::INFINITE);
                        reduction += 1;
                        delta += delta4() * delta / 128;
                    }
                    _ => {
                        average[td.pv_index] = if average[td.pv_index] == Score::NONE {
                            score
                        } else {
                            (average[td.pv_index] + score) / 2
                        };

                        td.shared.best_stats[td.pv_index].fetch_max(
                            ((depth as u32) << 16) | (average[td.pv_index] + 32768) as u32,
                            Ordering::AcqRel,
                        );

                        break;
                    }
                }

                td.root_moves[td.pv_start..=td.pv_index].sort_by_key(|rm| std::cmp::Reverse(rm.score));

                if report == Report::Full && td.shared.nodes.aggregate() > 10_000_000 {
                    td.print_uci_info(depth);
                }
            }
        }

        if td.shared.status.get() != Status::STOPPED {
            td.completed_depth = depth;
        }

        if report == Report::Full
            && !(is_loss(td.root_moves[0].display_score) && td.shared.status.get() == Status::STOPPED)
            && (td.shared.status.get() == Status::STOPPED
                || td.pv_index + 1 == td.multi_pv
                || td.shared.nodes.aggregate() > 10_000_000)
        {
            td.print_uci_info(depth);
        }

        if (td.root_moves[0].score - average[td.pv_index]).abs() < 12 {
            eval_stability += 1;
        } else {
            eval_stability = 0;
        }

        if last_best_rootmove.mv == td.root_moves[0].mv {
            pv_stability += 1;
        } else {
            pv_stability = 0;
        }

        best_move_changes += td.best_move_changes;

        if td.root_moves[0].score != -Score::INFINITE
            && is_loss(td.root_moves[0].score)
            && td.shared.status.get() == Status::STOPPED
        {
            if let Some(pos) = td.root_moves.iter().position(|rm| rm.mv == last_best_rootmove.mv) {
                td.root_moves.remove(pos);
                td.root_moves.insert(0, last_best_rootmove.clone());
            }
        } else {
            last_best_rootmove = td.root_moves[0].clone();
        }

        if td.shared.status.get() == Status::STOPPED {
            break;
        }

        let multiplier = || {
            let nodes_factor = (2.7168 - 2.2669 * (td.root_moves[0].nodes as f32 / td.nodes() as f32)).max(0.5630_f32);

            let pv_stability = (1.25 - 0.05 * pv_stability as f32).max(0.85);

            let eval_stability = (1.2 - 0.04 * eval_stability as f32).max(0.88);

            let score_trend = (0.8 + 0.05 * (td.previous_best_score - td.root_moves[0].score) as f32).clamp(0.80, 1.45);

            let best_move_stability = 1.0 + best_move_changes as f32 / 4.0;

            nodes_factor * pv_stability * eval_stability * score_trend * best_move_stability
        };

        if td.time_manager.soft_limit(td, multiplier) {
            if !soft_stop_voted {
                soft_stop_voted = true;

                let votes = td.shared.soft_stop_votes.fetch_add(1, Ordering::AcqRel) + 1;
                let majority = (thread_count * 65).div_ceil(100);
                if votes >= majority {
                    td.shared.status.set(Status::STOPPED);
                }
            }
        } else if soft_stop_voted {
            soft_stop_voted = false;
            td.shared.soft_stop_votes.fetch_sub(1, Ordering::AcqRel);
        }

        if td.shared.status.get() == Status::STOPPED {
            break;
        }
    }

    if report == Report::Minimal {
        td.print_uci_info(td.root_depth);
    }

    td.previous_best_score = td.root_moves[0].score;
}

fn search<NODE: NodeType>(
    td: &mut ThreadData, mut alpha: i32, mut beta: i32, depth: i32, cut_node: bool, ply: isize,
) -> i32 {
    debug_assert!(ply as usize <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);
    debug_assert!(NODE::PV || alpha == beta - 1);

    let stm = td.board.side_to_move();
    let in_check = td.board.in_check();
    let excluded = td.stack[ply].excluded.is_present();

    if !NODE::ROOT && NODE::PV {
        td.pv_table.clear(ply as usize);
    }

    if td.shared.status.get() == Status::STOPPED {
        return Score::ZERO;
    }

    // Qsearch Dive
    if depth <= 0 {
        return qsearch::<NODE>(td, alpha, beta, ply);
    }

    let draw_score = draw(td);
    if !NODE::ROOT && alpha < draw_score && td.board.upcoming_repetition(ply as usize) {
        alpha = draw_score;
        if alpha >= beta {
            return alpha;
        }
    }

    if NODE::PV {
        td.sel_depth = td.sel_depth.max(ply as i32);
    }

    if td.id == 0 && td.time_manager.check_time(td) {
        td.shared.status.set(Status::STOPPED);
        return Score::ZERO;
    }

    if !NODE::ROOT {
        if td.board.is_draw(ply) {
            return draw(td);
        }

        if ply as usize >= MAX_PLY - 1 {
            return if in_check { draw(td) } else { td.nnue.evaluate(&td.board) };
        }

        // Mate Distance Pruning (MDP)
        alpha = alpha.max(mated_in(ply));
        beta = beta.min(mate_in(ply + 1));

        if alpha >= beta {
            return alpha;
        }
    }

    #[cfg(feature = "syzygy")]
    let mut max_score = Score::INFINITE;

    let mut best_score = -Score::INFINITE;

    let mut depth = depth.min(MAX_PLY as i32 - 1);

    let hash = td.board.hash();
    let entry = td.shared.tt.read(hash, td.board.halfmove_clock(), ply);

    let mut tt_depth = 0;
    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;
    let mut tt_pv = NODE::PV;

    // Search early TT cutoff
    if let Some(entry) = &entry {
        tt_depth = entry.depth;
        tt_move = entry.mv;
        tt_score = entry.score;
        tt_bound = entry.bound;
        tt_pv |= entry.tt_pv;

        if !NODE::PV
            && !excluded
            && tt_depth > depth - (tt_score < beta) as i32
            && is_valid(tt_score)
            && match tt_bound {
                Bound::Upper => tt_score <= alpha && (!cut_node || depth > 5),
                Bound::Lower => tt_score >= beta && (cut_node || depth > 5),
                _ => true,
            }
        {
            if tt_move.is_quiet() && tt_score >= beta && td.stack[ply - 1].move_count < ttcut7() {
                let quiet_bonus = (ttcut1() * depth - ttcut2()).min(ttcut3());
                let cont_bonus = (ttcut4() * depth - ttcut5()).min(ttcut6());

                td.quiet_history.update(td.board.all_threats(), stm, tt_move, quiet_bonus);
                update_continuation_histories(td, ply, td.board.moved_piece(tt_move), tt_move.to(), cont_bonus);
            }

            if td.board.halfmove_clock() < 90 {
                return tt_score;
            }
        }
    }

    // Tablebases Probe
    #[cfg(feature = "syzygy")]
    if !NODE::ROOT
        && !excluded
        && !td.stop_probing_tb
        && td.board.halfmove_clock() == 0
        && td.board.castling().raw() == 0
        && td.board.occupancies().popcount() <= tb::size()
        && let Some(outcome) = tb::probe(&td.board)
    {
        td.shared.tb_hits.increment(td.id);

        let (score, bound) = match outcome {
            tb::GameOutcome::Win => (tb_win_in(ply), Bound::Lower),
            tb::GameOutcome::Loss => (tb_loss_in(ply), Bound::Upper),
            tb::GameOutcome::Draw => (Score::ZERO, Bound::Exact),
        };

        if bound == Bound::Exact
            || (bound == Bound::Lower && score >= beta)
            || (bound == Bound::Upper && score <= alpha)
        {
            let depth = (depth + 6).min(MAX_PLY as i32 - 1);
            td.shared.tt.write(hash, depth, Score::NONE, score, bound, Move::NULL, ply, tt_pv, false);
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

    let correction_value = eval_correction(td, ply);

    let raw_eval;
    let mut eval;

    // Evaluation
    if in_check {
        raw_eval = Score::NONE;
        eval = Score::NONE;
    } else if excluded {
        raw_eval = Score::NONE;
        eval = td.stack[ply].eval;
    } else if let Some(entry) = &entry {
        raw_eval = if is_valid(entry.raw_eval) { entry.raw_eval } else { td.nnue.evaluate(&td.board) };
        eval = correct_eval(td, raw_eval, correction_value);
    } else {
        raw_eval = td.nnue.evaluate(&td.board);
        eval = correct_eval(td, raw_eval, correction_value);

        td.shared.tt.write(hash, TtDepth::SOME, raw_eval, Score::NONE, Bound::None, Move::NULL, ply, tt_pv, false);
    }

    // Prefer the TT entry to tighten the evaluation when its bound aligns with
    // the current alpha-beta window; otherwise, retain the unbounded evaluation
    let mut estimated_score = eval;

    if !in_check
        && !excluded
        && is_valid(tt_score)
        && match tt_bound {
            Bound::Upper => tt_score < eval,
            Bound::Lower => tt_score > eval,
            _ => true,
        }
    {
        estimated_score = tt_score;
    }

    // Use the bounded TT entry score for evaluation when in check
    if in_check
        && !is_decisive(tt_score)
        && is_valid(tt_score)
        && match tt_bound {
            Bound::Upper => tt_score <= alpha,
            Bound::Lower => tt_score >= beta,
            _ => true,
        }
    {
        estimated_score = tt_score;
        eval = tt_score;
    }

    td.stack[ply].eval = eval;
    td.stack[ply].tt_move = tt_move;
    td.stack[ply].tt_pv = tt_pv;
    td.stack[ply].reduction = 0;
    td.stack[ply].move_count = 0;
    td.stack[ply + 2].cutoff_count = 0;

    // Quiet move ordering using eval difference
    if !NODE::ROOT && !in_check && !excluded && td.stack[ply - 1].mv.is_quiet() && is_valid(td.stack[ply - 1].eval) {
        let value = evalord1() * (-(eval + td.stack[ply - 1].eval)) / 128;
        let bonus = value.clamp(-evalord2(), evalord3());

        td.quiet_history.update(td.board.prior_threats(), !stm, td.stack[ply - 1].mv, bonus);
    }

    // Hindsight reductions
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[ply - 1].reduction >= hs1()
        && eval + td.stack[ply - 1].eval < 0
    {
        depth += 1;
    }

    if !NODE::ROOT
        && !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.stack[ply - 1].reduction > 0
        && is_valid(td.stack[ply - 1].eval)
        && eval + td.stack[ply - 1].eval > hs2()
    {
        depth -= 1;
    }

    let potential_singularity = depth >= 5 + tt_pv as i32
        && tt_depth >= depth - 3
        && tt_bound != Bound::Upper
        && is_valid(tt_score)
        && !is_decisive(tt_score);

    let mut improvement = 0;

    if is_valid(td.stack[ply - 2].eval) && !in_check {
        improvement = eval - td.stack[ply - 2].eval;
    } else if is_valid(td.stack[ply - 4].eval) && !in_check {
        improvement = eval - td.stack[ply - 4].eval;
    }

    let improving = improvement > 0;

    // Razoring
    if !NODE::PV
        && !in_check
        && estimated_score < alpha - razor1() - razor2() * depth * depth
        && alpha < 2048
        && !tt_move.is_quiet()
    {
        return qsearch::<NonPV>(td, alpha, beta, ply);
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && !excluded
        && is_valid(estimated_score)
        && estimated_score >= beta
        && estimated_score
            >= beta + rfp1() * depth * depth / 128 + rfp2() * depth - (rfp3() * improving as i32)
                + rfp4() * correction_value.abs() / 1024
                - rfp5() * ((td.board.all_threats() & td.board.colors(stm)).is_empty() && !td.board.in_check()) as i32
                + rfp6()
        && !is_loss(beta)
        && !is_win(estimated_score)
    {
        return beta + (estimated_score - beta) / 3;
    }

    // Null Move Pruning (NMP)
    if cut_node
        && !in_check
        && !excluded
        && !potential_singularity
        && estimated_score >= beta
        && estimated_score
            >= beta - nmp1() * depth + nmp2() * tt_pv as i32 - nmp3() * improvement / 1024 + nmp4()
                - nmp5() * (td.stack[ply + 1].cutoff_count < 2) as i32
        && ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
        && !is_loss(beta)
        && !(tt_bound == Bound::Lower
            && tt_move.is_present()
            && tt_move.is_capture()
            && td.board.piece_on(tt_move.to()).value() >= PieceType::Knight.value())
    {
        debug_assert_ne!(td.stack[ply - 1].mv, Move::NULL);

        let r = (nmp6() + nmp7() * depth + nmp8() * (estimated_score - beta).clamp(0, nmp9()) / 128) / 1024;

        td.stack[ply].conthist = td.stack.sentinel().conthist;
        td.stack[ply].contcorrhist = td.stack.sentinel().contcorrhist;
        td.stack[ply].piece = Piece::None;
        td.stack[ply].mv = Move::NULL;

        td.board.make_null_move();
        td.shared.tt.prefetch(td.board.hash());

        let score = -search::<NonPV>(td, -beta, -beta + 1, depth - r, false, ply + 1);

        td.board.undo_null_move();

        if td.shared.status.get() == Status::STOPPED {
            return Score::ZERO;
        }

        if score >= beta && !is_win(score) {
            if td.nmp_min_ply > 0 || depth < 16 {
                return score;
            }

            td.nmp_min_ply = ply as i32 + 3 * (depth - r) / 4;
            let verified_score = search::<NonPV>(td, beta - 1, beta, depth - r, false, ply);
            td.nmp_min_ply = 0;

            if td.shared.status.get() == Status::STOPPED {
                return Score::ZERO;
            }

            if verified_score >= beta {
                return score;
            }
        }
    }

    // ProbCut
    let mut probcut_beta = beta + probcut1() - probcut2() * improving as i32;

    if cut_node
        && !is_decisive(beta)
        && if is_valid(tt_score) { tt_score >= probcut_beta && !is_decisive(tt_score) } else { eval >= beta }
        && !tt_move.is_quiet()
    {
        let mut move_picker = MovePicker::new_probcut(probcut_beta - eval);

        while let Some(mv) = move_picker.next::<NODE>(td, true, ply) {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if mv == td.stack[ply].excluded {
                continue;
            }

            make_move(td, ply, mv);

            let mut score = -qsearch::<NonPV>(td, -probcut_beta, -probcut_beta + 1, ply + 1);

            let base_depth = (depth - 4).max(0);
            let mut probcut_depth = (base_depth - (score - probcut_beta) / probcut3()).clamp(0, base_depth);

            if score >= probcut_beta && probcut_depth > 0 {
                let adjusted_beta = (probcut_beta + probcut4() * (base_depth - probcut_depth)).min(Score::INFINITE);

                score = -search::<NonPV>(td, -adjusted_beta, -adjusted_beta + 1, probcut_depth, false, ply + 1);

                if score < adjusted_beta && probcut_beta < adjusted_beta {
                    probcut_depth = base_depth;
                    score = -search::<NonPV>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, false, ply + 1);
                } else {
                    probcut_beta = adjusted_beta;
                }
            }

            undo_move(td, mv);

            if td.shared.status.get() == Status::STOPPED {
                return Score::ZERO;
            }

            if score >= probcut_beta {
                td.shared.tt.write(hash, probcut_depth + 1, raw_eval, score, Bound::Lower, mv, ply, tt_pv, false);

                if !is_decisive(score) {
                    return (3 * score + beta) / 4;
                }
            }
        }
    }

    // Singular Extensions (SE)
    let mut extension = 0;

    if !NODE::ROOT && !excluded && potential_singularity {
        debug_assert!(is_valid(tt_score));

        let singular_margin = if tt_bound == Bound::Exact { (depth as u32).div_ceil(4) as i32 } else { depth }
            + depth * (tt_pv && !NODE::PV) as i32;
        let singular_beta = tt_score - singular_margin;
        let singular_depth = (depth - 1) / 2;

        td.stack[ply].excluded = tt_move;
        let score = search::<NonPV>(td, singular_beta - 1, singular_beta, singular_depth, cut_node, ply);
        td.stack[ply].excluded = Move::NULL;

        if td.shared.status.get() == Status::STOPPED {
            return Score::ZERO;
        }

        if score < singular_beta {
            let double_margin =
                se1() * NODE::PV as i32 - se2() * tt_move.is_quiet() as i32 - se3() * correction_value.abs() / 128;
            let triple_margin =
                se4() * NODE::PV as i32 - se5() * tt_move.is_quiet() as i32 - se6() * correction_value.abs() / 128
                    + se7();

            extension = 1;
            extension += (score < singular_beta - double_margin) as i32;
            extension += (score < singular_beta - triple_margin) as i32;
        }
        // Multi-Cut
        else if score >= beta && !is_decisive(score) {
            return (2 * score + beta) / 3;
        }
        // Negative Extensions
        else if tt_score >= beta {
            extension = -2;
        } else if cut_node {
            extension = -2;
        }
    }

    let mut best_move = Move::NULL;
    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(tt_move);
    let mut skip_quiets = false;
    let mut current_search_count = 0;
    let mut alpha_raises = 0;

    while let Some(mv) = move_picker.next::<NODE>(td, skip_quiets, ply) {
        if mv == td.stack[ply].excluded {
            continue;
        }

        if NODE::ROOT && !td.root_moves[td.pv_index..td.pv_end].iter().any(|rm| rm.mv == mv) {
            continue;
        }

        move_count += 1;
        current_search_count = 0;
        td.stack[ply].move_count = move_count;

        let is_quiet = mv.is_quiet();

        let history = if is_quiet {
            td.quiet_history.get(td.board.all_threats(), stm, mv) + td.conthist(ply, 1, mv) + td.conthist(ply, 2, mv)
        } else {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.get(td.board.all_threats(), td.board.moved_piece(mv), mv.to(), captured)
        };

        if !NODE::ROOT && !is_loss(best_score) {
            // Late Move Pruning (LMP)
            if !in_check
                && !td.board.is_direct_check(mv)
                && is_quiet
                && move_count
                    >= (lmp1() + lmp2() * improvement / 16 + lmp3() * depth * depth + lmp4() * history / 1024) / 1024
            {
                skip_quiets = true;
                continue;
            }

            // Futility Pruning (FP)
            let futility_value = eval + fp1() * depth + fp2() * history / 1024 + fp3() * (eval >= beta) as i32 - fp4();

            if !in_check && is_quiet && depth < fp5() && futility_value <= alpha && !td.board.is_direct_check(mv) {
                if !is_decisive(best_score) && best_score < futility_value {
                    best_score = futility_value;
                }
                skip_quiets = true;
                continue;
            }

            // Bad Noisy Futility Pruning (BNFP)
            let noisy_futility_value = eval + bnfp1() * depth + bnfp2() * history / 1024 + bnfp3();

            if !in_check
                && depth < bnfp4()
                && move_picker.stage() == Stage::BadNoisy
                && noisy_futility_value <= alpha
                && !td.board.is_direct_check(mv)
            {
                if !is_decisive(best_score) && best_score < noisy_futility_value {
                    best_score = noisy_futility_value;
                }
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet {
                (-see1() * depth * depth + see2() * depth - see3() * history / 1024 + see4()).min(0)
            } else {
                (-see5() * depth * depth - see6() * depth - see7() * history / 1024 + see8()).min(0)
            };

            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        let initial_nodes = td.nodes();

        make_move(td, ply, mv);

        let mut new_depth = if move_count == 1 { depth + extension - 1 } else { depth + (extension > 0) as i32 - 1 };

        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 2 && move_count >= 2 {
            let mut reduction = lmr1() * (move_count.ilog2() * depth.ilog2()) as i32;

            reduction -= lmr2() * move_count;
            reduction -= lmr3() * correction_value.abs() / 1024;
            reduction += lmr4() * alpha_raises;

            reduction += lmr5() * (is_valid(tt_score) && tt_score <= alpha) as i32;
            reduction += lmr6() * (is_valid(tt_score) && tt_depth < depth) as i32;

            if is_quiet {
                reduction += lmr7();
                reduction -= lmr8() * history / 1024;
            } else {
                reduction += lmr9();
                reduction -= lmr10() * history / 1024;
            }

            if NODE::PV {
                reduction -= lmr11() + lmr12() * (beta - alpha) / td.root_delta;
            }

            if tt_pv {
                reduction -= lmr13();
                reduction -= lmr14() * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= lmr15() * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if !tt_pv && cut_node {
                reduction += lmr16();
                reduction += lmr17() * tt_move.is_null() as i32;
            }

            if !improving {
                reduction += (lmr18() - lmr19() * improvement / 128).min(lmr20());
            }

            if td.board.in_check() {
                reduction -= lmr21();
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += lmr22();
            }

            if !NODE::PV && td.stack[ply - 1].reduction > reduction + 512 {
                reduction += lmr23();
            }

            let reduced_depth =
                (new_depth - reduction / 1024).clamp(1, new_depth + (move_count <= 3) as i32 + 1) + 2 * NODE::PV as i32;

            td.stack[ply].reduction = reduction;
            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, true, ply + 1);
            td.stack[ply].reduction = 0;
            current_search_count += 1;

            if score > alpha {
                if !NODE::ROOT {
                    new_depth += (score > best_score + dod1()) as i32;
                    new_depth += (score > best_score + dod2()) as i32;
                    new_depth -= (score < best_score + dos1() + reduced_depth) as i32;
                }

                if new_depth > reduced_depth {
                    score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node, ply + 1);
                    current_search_count += 1;
                }
            }
        }
        // Full Depth Search (FDS)
        else if !NODE::PV || move_count > 1 {
            let mut reduction = fds1() * (move_count.ilog2() * depth.ilog2()) as i32;

            reduction -= fds2() * move_count;
            reduction -= fds3() * correction_value.abs() / 1024;

            if is_quiet {
                reduction += fds4();
                reduction -= fds5() * history / 1024;
            } else {
                reduction += fds6();
                reduction -= fds7() * history / 1024;
            }

            if tt_pv {
                reduction -= fds8();
                reduction -= fds9() * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if !tt_pv && cut_node {
                reduction += fds10();
                reduction += fds11() * tt_move.is_null() as i32;
            }

            if !improving {
                reduction += (fds12() - fds13() * improvement / 128).min(fds14());
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += fds15();
            }

            if mv == tt_move {
                reduction -= fds16();
            }

            if !NODE::PV && td.stack[ply - 1].reduction > reduction + 512 {
                reduction += fds17();
            }

            let reduced_depth =
                new_depth - (reduction >= fdsred1()) as i32 - (reduction >= fdsred2() && new_depth >= 3) as i32;

            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, !cut_node, ply + 1);
            current_search_count += 1;
        }

        // Principal Variation Search (PVS)
        if NODE::PV && (move_count == 1 || score > alpha) {
            if mv == tt_move && tt_depth > 1 && td.root_depth > 8 {
                new_depth = new_depth.max(1);
            }

            score = -search::<PV>(td, -beta, -alpha, new_depth, false, ply + 1);
            current_search_count += 1;
        }

        undo_move(td, mv);

        if td.shared.status.get() == Status::STOPPED {
            return Score::ZERO;
        }

        if NODE::ROOT {
            let current_nodes = td.nodes();
            let root_move = td.root_moves.iter_mut().find(|v| v.mv == mv).unwrap();

            root_move.nodes += current_nodes - initial_nodes;

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

                if move_count > 1 && td.pv_index == 0 {
                    td.best_move_changes += 1;
                }
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
                    td.pv_table.update(ply as usize, mv);
                }

                if score >= beta {
                    bound = Bound::Lower;
                    td.stack[ply].cutoff_count += 1;
                    break;
                }

                alpha = score;

                if !is_decisive(score) {
                    alpha_raises += 1;
                }
            }
        }

        if mv != best_move && move_count < 32 {
            if is_quiet {
                quiet_moves.push(mv);
            } else {
                noisy_moves.push(mv);
            }
        }
    }

    if move_count == 0 {
        if excluded {
            return -Score::TB_WIN_IN_MAX + 1;
        }

        return if in_check { mated_in(ply) } else { draw(td) };
    }

    if best_move.is_present() {
        let noisy_bonus = (noisy1() * depth).min(noisy3()) - noisy2() - noisy4() * cut_node as i32;
        let noisy_malus = (noisy5() * depth).min(noisy7()) - noisy6() - noisy8() * noisy_moves.len() as i32;

        let quiet_bonus = (quiet1() * depth).min(quiet3()) - quiet2() - quiet4() * cut_node as i32;
        let quiet_malus = (quiet5() * depth).min(quiet7()) - quiet6() - quiet8() * quiet_moves.len() as i32;

        let cont_bonus = (cont1() * depth).min(cont3()) - cont2() - cont4() * cut_node as i32;
        let cont_malus = (cont5() * depth).min(cont7()) - cont6() - cont8() * quiet_moves.len() as i32;

        if best_move.is_noisy() {
            td.noisy_history.update(
                td.board.all_threats(),
                td.board.moved_piece(best_move),
                best_move.to(),
                td.board.piece_on(best_move.to()).piece_type(),
                noisy_bonus,
            );
        } else {
            td.quiet_history.update(td.board.all_threats(), stm, best_move, quiet_bonus);
            update_continuation_histories(td, ply, td.board.moved_piece(best_move), best_move.to(), cont_bonus);

            for &mv in quiet_moves.iter() {
                td.quiet_history.update(td.board.all_threats(), stm, mv, -quiet_malus);
                update_continuation_histories(td, ply, td.board.moved_piece(mv), mv.to(), -cont_malus);
            }
        }

        for &mv in noisy_moves.iter() {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.update(td.board.all_threats(), td.board.moved_piece(mv), mv.to(), captured, -noisy_malus);
        }

        if !NODE::ROOT && td.stack[ply - 1].mv.is_quiet() && td.stack[ply - 1].move_count < 2 {
            let malus = (refut1() * depth - refut2()).min(refut3());
            update_continuation_histories(td, ply - 1, td.stack[ply - 1].piece, td.stack[ply - 1].mv.to(), -malus);
        }

        if current_search_count > 1 && best_move.is_quiet() && best_score >= beta {
            let bonus = (post1() * depth - post2()).min(post3());
            update_continuation_histories(td, ply, td.stack[ply].piece, best_move.to(), bonus);
        }
    }

    if !NODE::ROOT && bound == Bound::Upper {
        let prior_move = td.stack[ply - 1].mv;
        if prior_move.is_quiet() {
            let mut factor = pcm1();
            factor += pcm2() * (td.stack[ply - 1].move_count > pcm3()) as i32;
            factor += pcm4() * (prior_move == td.stack[ply - 1].tt_move) as i32;
            factor += pcm5() * (!in_check && best_score <= eval - pcm6()) as i32;
            factor +=
                pcm7() * (is_valid(td.stack[ply - 1].eval) && best_score <= -td.stack[ply - 1].eval - pcm8()) as i32;

            let scaled_bonus = factor * (pcm9() * depth - pcm10()).min(pcm11()) / 128;

            td.quiet_history.update(td.board.prior_threats(), !stm, prior_move, scaled_bonus);

            let entry = &td.stack[ply - 2];
            if entry.mv.is_present() {
                let bonus = (pcm12() * depth - pcm13()).min(pcm14());
                td.continuation_history.update(entry.conthist, td.stack[ply - 1].piece, prior_move.to(), bonus);
            }
        } else if prior_move.is_noisy() {
            let captured = td.board.captured_piece().unwrap_or_default().piece_type();
            let bonus = pcm15();

            td.noisy_history.update(
                td.board.prior_threats(),
                td.board.piece_on(prior_move.to()),
                prior_move.to(),
                captured,
                bonus,
            );
        }
    }

    tt_pv |= !NODE::ROOT && bound == Bound::Upper && move_count > 2 && td.stack[ply - 1].tt_pv;

    if !NODE::ROOT && best_score >= beta && !is_decisive(best_score) && !is_decisive(alpha) {
        let weight = depth.min(8);
        best_score = (best_score * weight + beta) / (weight + 1);
    }

    #[cfg(feature = "syzygy")]
    if NODE::PV {
        best_score = best_score.min(max_score);
    }

    if !(excluded || NODE::ROOT && td.pv_index > 0) {
        td.shared.tt.write(hash, depth, raw_eval, best_score, bound, best_move, ply, tt_pv, NODE::PV);
    }

    if !(in_check
        || best_move.is_noisy()
        || (bound == Bound::Upper && best_score >= eval)
        || (bound == Bound::Lower && best_score <= eval))
    {
        update_correction_histories(td, depth, best_score - eval, ply);
    }

    debug_assert!(alpha < beta);
    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn qsearch<NODE: NodeType>(td: &mut ThreadData, mut alpha: i32, beta: i32, ply: isize) -> i32 {
    debug_assert!(!NODE::ROOT);
    debug_assert!(ply as usize <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);
    debug_assert!(NODE::PV || alpha == beta - 1);

    let draw_score = draw(td);
    if alpha < draw_score && td.board.upcoming_repetition(ply as usize) {
        alpha = draw_score;
        if alpha >= beta {
            return alpha;
        }
    }

    let stm = td.board.side_to_move();
    let in_check = td.board.in_check();

    if NODE::PV {
        td.pv_table.clear(ply as usize);
        td.sel_depth = td.sel_depth.max(ply as i32);
    }

    if td.id == 0 && td.time_manager.check_time(td) {
        td.shared.status.set(Status::STOPPED);
        return Score::ZERO;
    }

    if td.board.is_draw(ply) {
        return draw(td);
    }

    if ply as usize >= MAX_PLY - 1 {
        return if in_check { draw(td) } else { td.nnue.evaluate(&td.board) };
    }

    let hash = td.board.hash();
    let entry = td.shared.tt.read(hash, td.board.halfmove_clock(), ply);

    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;
    let mut tt_pv = NODE::PV;

    // QS early TT cutoff
    if let Some(entry) = &entry {
        tt_move = entry.mv;
        tt_score = entry.score;
        tt_bound = entry.bound;
        tt_pv |= entry.tt_pv;

        if is_valid(tt_score)
            && (!NODE::PV || !is_decisive(tt_score))
            && match tt_bound {
                Bound::Upper => tt_score <= alpha,
                Bound::Lower => tt_score >= beta,
                _ => true,
            }
        {
            return tt_score;
        }
    }

    let raw_eval;
    let eval;
    let mut best_score;

    // Evaluation
    if in_check {
        raw_eval = Score::NONE;
        eval = Score::NONE;
        best_score = -Score::INFINITE;
    } else {
        raw_eval = match &entry {
            Some(entry) if is_valid(entry.raw_eval) => entry.raw_eval,
            _ => td.nnue.evaluate(&td.board),
        };
        eval = correct_eval(td, raw_eval, eval_correction(td, ply));
        best_score = eval;

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
    }

    // Stand Pat
    if best_score >= beta {
        if !is_decisive(best_score) && !is_decisive(beta) {
            best_score = beta + (best_score - beta) / 3;
        }

        if entry.is_none() {
            td.shared.tt.write(hash, TtDepth::SOME, raw_eval, best_score, Bound::Lower, Move::NULL, ply, tt_pv, false);
        }

        return best_score;
    }

    if best_score > alpha {
        alpha = best_score;
    }

    let mut best_move = Move::NULL;

    let mut move_count = 0;
    let mut move_picker = MovePicker::new_qsearch();

    let skip_quiets =
        |best_score| !((in_check && is_loss(best_score)) || (tt_move.is_quiet() && tt_bound != Bound::Upper));

    while let Some(mv) = move_picker.next::<NODE>(td, skip_quiets(best_score), ply) {
        move_count += 1;

        if !is_loss(best_score) {
            // Late Move Pruning (LMP)
            if move_count >= 3 && !td.board.is_direct_check(mv) {
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            if is_valid(eval) && !td.board.see(mv, (alpha - eval) / qs1() - qs2()) {
                continue;
            }
        }

        make_move(td, ply, mv);

        let score = -qsearch::<NODE>(td, -beta, -alpha, ply + 1);

        undo_move(td, mv);

        if td.shared.status.get() == Status::STOPPED {
            return Score::ZERO;
        }

        if score > best_score {
            best_score = score;

            if score > alpha {
                best_move = mv;

                if NODE::PV {
                    td.pv_table.update(ply as usize, mv);
                }

                if score >= beta {
                    let bonus = if best_move.is_noisy() { 106 } else { 172 };

                    if best_move.is_noisy() {
                        td.noisy_history.update(
                            td.board.all_threats(),
                            td.board.moved_piece(best_move),
                            best_move.to(),
                            td.board.piece_on(best_move.to()).piece_type(),
                            bonus,
                        );
                    } else {
                        td.quiet_history.update(td.board.all_threats(), stm, best_move, bonus);
                    }

                    break;
                }

                alpha = score;
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

    td.shared.tt.write(hash, TtDepth::SOME, raw_eval, best_score, bound, best_move, ply, tt_pv, false);

    debug_assert!(alpha < beta);
    debug_assert!(-Score::INFINITE < best_score && best_score < Score::INFINITE);

    best_score
}

fn eval_correction(td: &ThreadData, ply: isize) -> i32 {
    let stm = td.board.side_to_move();
    let corrhist = td.corrhist();

    (corrhist.pawn.get(stm, td.board.pawn_key())
        + corrhist.minor.get(stm, td.board.minor_key())
        + corrhist.non_pawn[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + corrhist.non_pawn[Color::Black].get(stm, td.board.non_pawn_key(Color::Black))
        + td.continuation_corrhist.get(
            td.stack[ply - 2].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
        )
        + td.continuation_corrhist.get(
            td.stack[ply - 4].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
        ))
        / corrhist1()
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32, ply: isize) {
    let stm = td.board.side_to_move();
    let corrhist = td.corrhist();
    let bonus = (corrhist2() * depth * diff / 128).clamp(-corrhist3(), corrhist4());

    corrhist.pawn.update(stm, td.board.pawn_key(), bonus);
    corrhist.minor.update(stm, td.board.minor_key(), bonus);

    corrhist.non_pawn[Color::White].update(stm, td.board.non_pawn_key(Color::White), bonus);
    corrhist.non_pawn[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), bonus);

    if td.stack[ply - 1].mv.is_present() && td.stack[ply - 2].mv.is_present() {
        td.continuation_corrhist.update(
            td.stack[ply - 2].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
            bonus,
        );
    }

    if td.stack[ply - 1].mv.is_present() && td.stack[ply - 4].mv.is_present() {
        td.continuation_corrhist.update(
            td.stack[ply - 4].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
            bonus,
        );
    }
}

fn update_continuation_histories(td: &mut ThreadData, ply: isize, piece: Piece, sq: Square, bonus: i32) {
    for offset in [1, 2, 4, 6] {
        let entry = &td.stack[ply - offset];
        if entry.mv.is_present() {
            td.continuation_history.update(entry.conthist, piece, sq, bonus);
        }
    }
}

fn make_move(td: &mut ThreadData, ply: isize, mv: Move) {
    td.stack[ply].mv = mv;
    td.stack[ply].piece = td.board.moved_piece(mv);
    td.stack[ply].conthist =
        td.continuation_history.subtable_ptr(td.board.in_check(), mv.is_noisy(), td.board.moved_piece(mv), mv.to());
    td.stack[ply].contcorrhist =
        td.continuation_corrhist.subtable_ptr(td.board.in_check(), mv.is_noisy(), td.board.moved_piece(mv), mv.to());

    td.shared.nodes.increment(td.id);

    td.nnue.push(mv, &td.board);
    td.board.make_move(mv, &mut td.nnue);

    td.shared.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.nnue.pop();
    td.board.undo_move(mv);
}
