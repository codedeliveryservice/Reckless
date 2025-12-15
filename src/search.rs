use crate::{
    evaluation::correct_eval,
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
    td.nnue.full_refresh(&td.board);

    td.root_moves = td
        .board
        .generate_all_moves()
        .iter()
        .filter(|v| td.board.is_legal(v.mv))
        .map(|v| RootMove { mv: v.mv, ..Default::default() })
        .collect();

    td.root_in_tb = false;
    td.stop_probing_tb = false;

    if td.board.castling().raw() == 0 && td.board.occupancies().len() <= tb_size() {
        tb_rank_rootmoves(td);
    }

    td.multi_pv = td.multi_pv.min(td.root_moves.len() as i32);

    let mut average = vec![Score::NONE; td.multi_pv as usize];
    let mut last_best_rootmove = RootMove::default();

    let mut eval_stability = 0;
    let mut pv_stability = 0;
    let mut best_move_changes = 0;

    // Iterative Deepening
    for depth in 1..MAX_PLY as i32 {
        best_move_changes /= 2;

        td.sel_depth = 0;
        td.root_depth = depth;
        td.best_move_changes = 0;

        let mut alpha = -Score::INFINITE;
        let mut beta = Score::INFINITE;

        let mut delta = 13;
        let mut reduction = 0;

        for rm in &mut td.root_moves {
            rm.previous_score = rm.score;
        }

        td.pv_start = 0;
        td.pv_end = 0;

        for index in 0..td.multi_pv {
            td.pv_index = index as usize;

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
            if depth >= 2 {
                delta += average[td.pv_index] * average[td.pv_index] / 23682;

                alpha = (average[td.pv_index] - delta).max(-Score::INFINITE);
                beta = (average[td.pv_index] + delta).min(Score::INFINITE);

                td.optimism[td.board.side_to_move()] = 158 * average[td.pv_index] / (average[td.pv_index].abs() + 191);
                td.optimism[!td.board.side_to_move()] = -td.optimism[td.board.side_to_move()];
            }

            loop {
                td.stack = Default::default();
                td.root_delta = beta - alpha;

                // Root Search
                let score = search::<Root>(td, alpha, beta, (depth - reduction).max(1), false, 0);

                td.root_moves[td.pv_index..td.pv_end].sort_by(|a, b| b.score.cmp(&a.score));

                if td.stopped {
                    break;
                }

                match score {
                    s if s <= alpha => {
                        beta = (3 * alpha + beta) / 4;
                        alpha = (score - delta).max(-Score::INFINITE);
                        reduction = 0;
                        delta += 26 * delta / 128;
                    }
                    s if s >= beta => {
                        alpha = (beta - delta).max(alpha);
                        beta = (score + delta).min(Score::INFINITE);
                        reduction += 1;
                        delta += 61 * delta / 128;
                    }
                    _ => {
                        average[td.pv_index] = if average[td.pv_index] == Score::NONE {
                            score
                        } else {
                            (average[td.pv_index] + score) / 2
                        };
                        break;
                    }
                }

                td.root_moves[td.pv_start..td.pv_index + 1].sort_by(|a, b| b.score.cmp(&a.score));

                if report == Report::Full && td.shared.nodes.aggregate() > 10_000_000 {
                    td.print_uci_info(depth);
                }
            }
        }

        if !td.stopped {
            td.completed_depth = depth;
        }

        if report == Report::Full
            && !(is_loss(td.root_moves[0].display_score) && td.stopped)
            && (td.stopped || td.pv_index as i32 + 1 == td.multi_pv || td.shared.nodes.aggregate() > 10_000_000)
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

        let multiplier = || {
            let nodes_factor = 2.15 - 1.5 * (td.root_moves[0].nodes as f32 / td.nodes() as f32);

            let pv_stability = (1.25 - 0.05 * pv_stability as f32).max(0.85);

            let eval_stability = (1.2 - 0.04 * eval_stability as f32).max(0.88);

            let score_trend = (0.8 + 0.05 * (td.previous_best_score - td.root_moves[0].score) as f32).clamp(0.80, 1.45);

            let recapture_factor = if td.root_moves[0].mv.to() == td.board.recapture_square() { 0.9 } else { 1.0 };

            let best_move_stability = 1.0 + best_move_changes as f32 / 4.0;

            nodes_factor * pv_stability * eval_stability * score_trend * recapture_factor * best_move_stability
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
    td: &mut ThreadData, mut alpha: i32, mut beta: i32, depth: i32, cut_node: bool, ply: isize,
) -> i32 {
    debug_assert!(ply as usize <= MAX_PLY);
    debug_assert!(-Score::INFINITE <= alpha && alpha < beta && beta <= Score::INFINITE);

    let in_check = td.board.in_check();
    let excluded = td.stack[ply].excluded.is_some();

    if !NODE::ROOT && NODE::PV {
        td.pv_table.clear(ply as usize);
    }

    if td.stopped {
        return Score::ZERO;
    }

    // Qsearch Dive
    if depth <= 0 {
        return qsearch::<NODE>(td, alpha, beta, ply);
    }

    if !NODE::ROOT && alpha < Score::ZERO && td.board.upcoming_repetition(ply as usize) {
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

        if ply as usize >= MAX_PLY - 1 {
            return if in_check { Score::DRAW } else { td.nnue.evaluate(&td.board) };
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
    let entry = td.shared.tt.read(hash, td.board.halfmove_clock(), ply);

    let mut tt_depth = 0;
    let mut tt_move = Move::NULL;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;
    let mut tt_pv = NODE::PV;

    // Search Early TT-Cut
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
            if tt_move.is_quiet() && tt_score >= beta && td.stack[ply - 1].move_count < 4 {
                let quiet_bonus = (190 * depth - 80).min(1830);
                let conthist_bonus = (109 * depth - 56).min(1337);

                td.quiet_history.update(td.board.threats(), td.board.side_to_move(), tt_move, quiet_bonus);
                update_continuation_histories(td, ply, td.board.moved_piece(tt_move), tt_move.to(), conthist_bonus);
            }

            if tt_score <= alpha && td.stack[ply - 1].move_count > 8 {
                let pcm_move = td.stack[ply - 1].mv;
                if pcm_move.is_quiet() {
                    let mut factor = 94;
                    factor += 183 * (initial_depth > 5) as i32;
                    factor += 132 * (pcm_move == td.stack[ply - 1].tt_move) as i32;
                    factor +=
                        240 * (is_valid(td.stack[ply - 1].eval) && tt_score <= -td.stack[ply - 1].eval - 95) as i32;

                    let scaled_bonus = factor * (163 * initial_depth - 44).min(2389) / 128;

                    td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);

                    let entry = &td.stack[ply - 2];
                    if entry.mv.is_some() {
                        let bonus = (122 * initial_depth - 34).min(1723);
                        td.continuation_history.update(entry.conthist, td.stack[ply - 1].piece, pcm_move.to(), bonus);
                    }
                }
            }

            if td.board.halfmove_clock() < 90 {
                return tt_score;
            }
        }
    }

    // Tablebases Probe
    if !NODE::ROOT
        && !excluded
        && !td.stop_probing_tb
        && td.board.halfmove_clock() == 0
        && td.board.castling().raw() == 0
        && td.board.occupancies().len() <= tb_size()
    {
        if let Some(outcome) = tb_probe(&td.board) {
            td.shared.tb_hits.increment(td.id);

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

    // Quiet Move Ordering Using eval difference
    if !NODE::ROOT && !in_check && !excluded && td.stack[ply - 1].mv.is_quiet() && is_valid(td.stack[ply - 1].eval) {
        let value = 865 * (-(eval + td.stack[ply - 1].eval)) / 128;
        let bonus = value.clamp(-119, 325);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[ply - 1].mv, bonus);
    }

    // Hindsight reductions
    if !NODE::ROOT && !in_check && !excluded && td.stack[ply - 1].reduction >= 2325 && eval + td.stack[ply - 1].eval < 0
    {
        depth += 1;
    }

    if !NODE::ROOT
        && !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.stack[ply - 1].reduction >= 758
        && is_valid(td.stack[ply - 1].eval)
        && eval + td.stack[ply - 1].eval > 62
    {
        depth -= 1;
    }

    let potential_singularity =
        depth >= 5 && tt_depth >= depth - 3 && tt_bound != Bound::Upper && is_valid(tt_score) && !is_decisive(tt_score);

    let mut improvement = 0;

    if is_valid(td.stack[ply - 2].eval) && !in_check {
        improvement = eval - td.stack[ply - 2].eval;
    } else if is_valid(td.stack[ply - 4].eval) && !in_check {
        improvement = eval - td.stack[ply - 4].eval;
    }

    let improving = improvement > 0;

    // Razoring
    if !NODE::PV && !in_check && estimated_score < alpha - 281 - 271 * depth * depth && alpha < 2048 {
        return qsearch::<NonPV>(td, alpha, beta, ply);
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && !excluded
        && is_valid(estimated_score)
        && estimated_score >= beta
        && estimated_score
            >= beta + 1085 * depth * depth / 128 + 25 * depth - (79 * improving as i32)
                + 500 * correction_value.abs() / 1024
                + 35 * (depth == 1) as i32
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
        && estimated_score >= eval
        && eval >= beta - 10 * depth + 128 * tt_pv as i32 - 133 * improvement / 1024 + 274
        && ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
        && !is_loss(beta)
    {
        debug_assert_ne!(td.stack[ply - 1].mv, Move::NULL);

        let r = (6582 + 273 * depth) / 1024;

        td.stack[ply].conthist = td.stack.sentinel().conthist;
        td.stack[ply].contcorrhist = td.stack.sentinel().contcorrhist;
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
    let probcut_beta = beta + 257 - 75 * improving as i32;

    if cut_node
        && depth >= 3
        && !is_decisive(beta)
        && (!is_valid(tt_score) || tt_score >= probcut_beta && !is_decisive(tt_score))
        && !tt_move.is_quiet()
    {
        let mut move_picker = MovePicker::new_probcut(probcut_beta - eval);

        let probcut_depth = (depth - 4).max(0);

        while let Some(mv) = move_picker.next::<NODE>(td, true, ply) {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if mv == td.stack[ply].excluded || !td.board.is_legal(mv) {
                continue;
            }

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
                td.shared.tt.write(hash, probcut_depth + 1, raw_eval, score, Bound::Lower, mv, ply, tt_pv, false);

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

    // Singular Extensions (SE)
    let mut extension = 0;

    if !NODE::ROOT && !excluded && potential_singularity && ply < 2 * td.root_depth as isize {
        debug_assert!(is_valid(tt_score));

        let singular_beta = tt_score - depth - depth * (tt_pv && !NODE::PV) as i32;
        let singular_depth = (depth - 1) / 2;

        td.stack[ply].excluded = tt_move;
        let score = search::<NonPV>(td, singular_beta - 1, singular_beta, singular_depth, cut_node, ply);
        td.stack[ply].excluded = Move::NULL;

        if td.stopped {
            return Score::ZERO;
        }

        if score < singular_beta {
            let double_margin = 2 + 272 * NODE::PV as i32;
            let triple_margin = 57 + 313 * NODE::PV as i32 - 14 * correction_value.abs() / 128;

            extension = 1;
            extension += (score < singular_beta - double_margin) as i32;
            extension += (score < singular_beta - triple_margin) as i32;

            if extension > 1 && depth < 14 {
                depth += 1;
            }
        }
        // Multi-Cut
        else if score >= beta && !is_decisive(score) {
            return (score * singular_depth + beta) / (singular_depth + 1);
        }
        // Negative-Extensions
        else if tt_score >= beta {
            extension = -2;
        } else if cut_node {
            extension = -2;
        }
    } else if NODE::PV && tt_move.is_noisy() && tt_move.to() == td.board.recapture_square() {
        extension = 1;
    }

    let mut best_move = Move::NULL;
    let mut bound = Bound::Upper;

    let mut quiet_moves = ArrayVec::<Move, 32>::new();
    let mut noisy_moves = ArrayVec::<Move, 32>::new();

    let mut move_count = 0;
    let mut move_picker = MovePicker::new(tt_move);
    let mut skip_quiets = false;
    let mut current_search_count = 0;

    while let Some(mv) = move_picker.next::<NODE>(td, skip_quiets, ply) {
        if mv == td.stack[ply].excluded || !td.board.is_legal(mv) {
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
            td.quiet_history.get(td.board.threats(), td.board.side_to_move(), mv)
                + td.conthist(ply, 1, mv)
                + td.conthist(ply, 2, mv)
        } else {
            let captured = td.board.piece_on(mv.to()).piece_type();
            td.noisy_history.get(td.board.threats(), td.board.moved_piece(mv), mv.to(), captured)
        };

        let mut reduction = (1209 + 285 * depth.ilog2() * move_count.ilog2()) as i32;

        if NODE::PV {
            reduction -= 425 + 453 * (beta - alpha) / td.root_delta;
        }

        if !improving {
            reduction += (443 - 268 * improvement / 128).min(1321);
        }

        if !NODE::ROOT && !is_loss(best_score) {
            let lmr_depth = (depth - reduction / 1024).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= !in_check
                && move_count
                    >= if improving || eval >= beta + 19 {
                        (3219 + 1093 * initial_depth * initial_depth) / 1024
                    } else {
                        (1252 + 320 * initial_depth * initial_depth) / 1024
                    };

            // Futility Pruning (FP)
            let futility_value = eval + 93 * lmr_depth + 62 * history / 1024 + 90 * (eval >= alpha) as i32 + 89;

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
            let noisy_futility_value = eval
                + 122 * lmr_depth
                + 70 * history / 1024
                + 84 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 1024
                + 79;

            if !in_check && lmr_depth < 6 && move_picker.stage() == Stage::BadNoisy && noisy_futility_value <= alpha {
                if !is_decisive(best_score) && best_score <= noisy_futility_value {
                    best_score = noisy_futility_value;
                }
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet {
                -1746 * lmr_depth * lmr_depth / 128 - 33 * history / 1024 + 24
            } else {
                -86 * depth - 32 * history / 1024 + 42
            };

            if !td.board.see(mv, threshold) {
                continue;
            }
        }

        let initial_nodes = td.nodes();

        make_move(td, ply, mv);

        let mut new_depth = if move_count == 1 { depth + extension - 1 } else { depth - 1 };
        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 2 && move_count > 1 {
            if is_quiet {
                reduction += 599;
                reduction -= 152 * history / 1024;
            } else {
                reduction += 355;
                reduction -= 102 * history / 1024;
                reduction -= 50 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128;
            }

            reduction -= 3326 * correction_value.abs() / 1024;
            reduction -= 68 * move_count;

            if tt_pv {
                reduction -= 349;
                reduction -= 714 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 897 * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if mv.is_noisy() && mv.to() == td.board.recapture_square() {
                reduction -= 907;
            }

            if !tt_pv && cut_node {
                reduction += 1713;
                reduction += 1086 * tt_move.is_null() as i32;
            }

            if td.board.in_check() || !td.board.has_non_pawns() {
                reduction -= 884;
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += 1498;
            }

            if is_valid(tt_score) && tt_score < alpha && tt_bound == Bound::Upper {
                reduction += 622;
            }

            if depth == 2 {
                reduction -= 1264;
            }

            let mut reduced_depth = (new_depth - reduction / 1024).clamp(1, new_depth + 1) + 2 * NODE::PV as i32;

            if NODE::PV && best_move.is_null() && (NODE::ROOT || tt_bound != Bound::Upper) && reduced_depth < new_depth
            {
                reduced_depth = new_depth;
            }

            td.stack[ply].reduction = reduction;
            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, true, ply + 1);
            td.stack[ply].reduction = 0;
            current_search_count += 1;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 43 + 482 * depth / 128) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node, ply + 1);
                    current_search_count += 1;
                }
            } else if score > alpha && score < best_score + 14 {
                new_depth -= 1;
            }
        }
        // Full Depth Search (FDS)
        else if !NODE::PV || move_count > 1 {
            if is_quiet {
                reduction += 406;
                reduction -= 154 * history / 1024;
            } else {
                reduction += 235;
                reduction -= 65 * history / 1024;
                reduction -= 47 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128;
            }

            reduction -= 2484 * correction_value.abs() / 1024;
            reduction -= 55 * move_count;

            if tt_pv {
                reduction -= 747;
                reduction -= 1080 * (is_valid(tt_score) && tt_depth >= depth) as i32;
            }

            if !tt_pv && cut_node {
                reduction += 1379;
                reduction += 1211 * tt_move.is_null() as i32;
            }

            if td.stack[ply + 1].cutoff_count > 2 {
                reduction += 1445;
            }

            if depth == 2 {
                reduction -= 1166;
            }

            if mv == tt_move {
                reduction -= 3187;
            }

            let reduced_depth = new_depth - (reduction >= 3072) as i32 - (reduction >= 5653 && new_depth >= 3) as i32;

            td.stack[ply].reduction = 1024 * ((initial_depth - 1) - new_depth);
            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, !cut_node, ply + 1);
            td.stack[ply].reduction = 0;
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

        if td.stopped {
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

                if depth > 2 && depth < 17 && !is_decisive(score) {
                    depth -= 1;
                }

                alpha = score;
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
            return alpha;
        }

        return if in_check { mated_in(ply) } else { Score::DRAW };
    }

    if best_move.is_some() {
        let bonus_noisy = (111 * depth - 54).min(861) - 77 * cut_node as i32;
        let malus_noisy = (173 * initial_depth - 53).min(1257) - 23 * noisy_moves.len() as i32;

        let bonus_quiet = (179 * depth - 75).min(1335) - 56 * cut_node as i32;
        let malus_quiet = (156 * initial_depth - 44).min(1056) - 41 * quiet_moves.len() as i32;

        let bonus_cont = (115 * depth - 67).min(972) - 50 * cut_node as i32;
        let malus_cont = (343 * initial_depth - 47).min(856) - 21 * quiet_moves.len() as i32;

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

        if !NODE::ROOT && td.stack[ply - 1].mv.is_quiet() && td.stack[ply - 1].move_count < 2 {
            let malus = (86 * initial_depth - 58).min(778);
            update_continuation_histories(td, ply - 1, td.stack[ply - 1].piece, td.stack[ply - 1].mv.to(), -malus);
        }

        if current_search_count > 1 && best_move.is_quiet() && best_score >= beta {
            let bonus = (210 * depth - 87).min(1663);
            update_continuation_histories(td, ply, td.stack[ply].piece, best_move.to(), bonus);
        }
    }

    if !NODE::ROOT && bound == Bound::Upper {
        let pcm_move = td.stack[ply - 1].mv;
        if pcm_move.is_quiet() {
            let mut factor = 97;
            factor += 159 * (initial_depth > 5) as i32;
            factor += 214 * (td.stack[ply - 1].move_count > 8) as i32;
            factor += 112 * (pcm_move == td.stack[ply - 1].tt_move) as i32;
            factor += 151 * (!in_check && best_score <= eval.min(raw_eval) - 95) as i32;
            factor += 319 * (is_valid(td.stack[ply - 1].eval) && best_score <= -td.stack[ply - 1].eval - 113) as i32;

            let scaled_bonus = factor * (157 * initial_depth - 33).min(2564) / 128;

            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);

            let entry = &td.stack[ply - 2];
            if entry.mv.is_some() {
                let bonus = (166 * initial_depth - 37).min(1268);
                td.continuation_history.update(entry.conthist, td.stack[ply - 1].piece, pcm_move.to(), bonus);
            }
        }
    }

    tt_pv |= !NODE::ROOT && bound == Bound::Upper && move_count > 2 && td.stack[ply - 1].tt_pv;

    if best_score >= beta && !is_decisive(best_score) && !is_decisive(alpha) {
        best_score = (best_score * depth + beta) / (depth + 1);
    }

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

    if alpha < Score::ZERO && td.board.upcoming_repetition(ply as usize) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    let in_check = td.board.in_check();

    if NODE::PV {
        td.pv_table.clear(ply as usize);
        td.sel_depth = td.sel_depth.max(ply as i32);
    }

    if td.time_manager.check_time(td) {
        td.stopped = true;
        return Score::ZERO;
    }

    if td.board.is_draw(ply) {
        return Score::DRAW;
    }

    if ply as usize >= MAX_PLY - 1 {
        return if in_check { Score::DRAW } else { td.nnue.evaluate(&td.board) };
    }

    let hash = td.board.hash();
    let entry = td.shared.tt.read(hash, td.board.halfmove_clock(), ply);

    let mut tt_pv = NODE::PV;
    let mut tt_score = Score::NONE;
    let mut tt_bound = Bound::None;

    // QS Early TT-Cut
    if let Some(entry) = &entry {
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

    let mut best_score = -Score::INFINITE;
    let mut raw_eval = Score::NONE;

    // Evaluation
    if !in_check {
        raw_eval = match &entry {
            Some(entry) if is_valid(entry.raw_eval) => entry.raw_eval,
            _ => td.nnue.evaluate(&td.board),
        };
        best_score = correct_eval(td, raw_eval, eval_correction(td, ply));

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
                td.shared.tt.write(
                    hash,
                    TtDepth::SOME,
                    raw_eval,
                    best_score,
                    Bound::Lower,
                    Move::NULL,
                    ply,
                    tt_pv,
                    false,
                );
            }

            return best_score;
        }

        if best_score > alpha {
            alpha = best_score;
        }
    }

    let mut best_move = Move::NULL;

    let mut move_count = 0;
    let mut move_picker = MovePicker::new_qsearch();

    while let Some(mv) = move_picker.next::<NODE>(td, !in_check, ply) {
        if !td.board.is_legal(mv) {
            continue;
        }

        move_count += 1;

        if !is_loss(best_score) && mv.to() != td.board.recapture_square() {
            if move_picker.stage() == Stage::BadNoisy {
                break;
            }

            if move_count >= 3 && !td.board.might_give_check_if_you_squint(mv) {
                break;
            }

            if in_check && mv.is_quiet() {
                break;
            }

            let futility_score = best_score + 42 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 128 + 109;

            if !in_check && futility_score <= alpha && !td.board.see(mv, 1) {
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

                if NODE::PV {
                    td.pv_table.update(ply as usize, mv);
                }

                if score >= beta {
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

    (td.pawn_corrhist.get(stm, td.board.pawn_key())
        + td.minor_corrhist.get(stm, td.board.minor_key())
        + td.major_corrhist.get(stm, td.board.major_key())
        + td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black))
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
        / 88
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32, ply: isize) {
    let stm = td.board.side_to_move();
    let bonus = (140 * depth * diff / 128).clamp(-5042, 2895);

    td.pawn_corrhist.update(stm, td.board.pawn_key(), bonus);
    td.minor_corrhist.update(stm, td.board.minor_key(), bonus);
    td.major_corrhist.update(stm, td.board.major_key(), bonus);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), bonus);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), bonus);

    if td.stack[ply - 1].mv.is_some() && td.stack[ply - 2].mv.is_some() {
        td.continuation_corrhist.update(
            td.stack[ply - 2].contcorrhist,
            td.stack[ply - 1].piece,
            td.stack[ply - 1].mv.to(),
            bonus,
        );
    }

    if td.stack[ply - 1].mv.is_some() && td.stack[ply - 4].mv.is_some() {
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
        if entry.mv.is_some() {
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
    td.board.make_move(mv, |board, piece, square, add| td.nnue.push_threats(board, piece, square, add));

    td.shared.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.nnue.pop();
    td.board.undo_move(mv);
}
