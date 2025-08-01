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
    td.nodes.clear();
    td.tb_hits.clear();

    td.nnue.full_refresh(&td.board);

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
        if depth >= 2 {
            delta += average * average / 26802;

            alpha = (average - delta).max(-Score::INFINITE);
            beta = (average + delta).min(Score::INFINITE);

            td.optimism[td.board.side_to_move()] = 112 * average / (average.abs() + 235);
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

            delta += delta * (40 + 15 * reduction) / 128;
        }

        if td.stopped {
            break;
        }

        td.nodes.flush();
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
            let nodes_factor = 2.15 - 1.5 * (td.node_table.get(td.pv.best_move()) as f32 / td.nodes.local() as f32);

            let pv_stability = 1.25 - 0.05 * pv_stability as f32;

            let eval_stability = 1.2 - 0.04 * eval_stability as f32;

            let score_trend = (800 + 20 * (td.previous_best_score - td.best_score)).clamp(750, 1500) as f32 / 1000.0;

            nodes_factor * pv_stability * eval_stability * score_trend
        };

        if td.time_manager.soft_limit(td, multiplier) {
            break;
        }

        if report == Report::Full {
            td.print_uci_info(depth, td.best_score);
        }
    }

    if report != Report::None {
        td.print_uci_info(td.root_depth, td.best_score);
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

    // Qsearch Dive
    if depth <= 0 {
        return qsearch::<NODE>(td, alpha, beta);
    }

    if !NODE::ROOT && alpha < Score::ZERO && td.board.upcoming_repetition(td.ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
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
                let quiet_bonus = (134 * depth - 72).min(1380) + 69 * !cut_node as i32;
                let conthist_bonus = (100 * depth - 62).min(1415) + 69 * !cut_node as i32;

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
    td.stack[td.ply].reduction = 0;
    td.stack[td.ply + 2].cutoff_count = 0;

    // Quiet Move Ordering Using Static-Eval
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[td.ply - 1].mv.is_quiet()
        && is_valid(td.stack[td.ply - 1].static_eval)
    {
        let value = 709 * (-(static_eval + td.stack[td.ply - 1].static_eval)) / 128;
        let bonus = value.clamp(-59, 138);

        td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), td.stack[td.ply - 1].mv, bonus);
    }

    // Hindsight reductions
    if !NODE::ROOT
        && !in_check
        && !excluded
        && td.stack[td.ply - 1].reduction >= 2765
        && static_eval + td.stack[td.ply - 1].static_eval < 0
    {
        depth += 1;
    }

    if !NODE::ROOT
        && !tt_pv
        && !in_check
        && !excluded
        && depth >= 2
        && td.stack[td.ply - 1].reduction >= 914
        && is_valid(td.stack[td.ply - 1].static_eval)
        && static_eval + td.stack[td.ply - 1].static_eval > 59
    {
        depth -= 1;
    }

    let potential_singularity =
        depth >= 5 && tt_depth >= depth - 3 && tt_bound != Bound::Upper && is_valid(tt_score) && !is_decisive(tt_score);

    let mut improvement = 0;
    if !in_check && td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && is_valid(td.stack[td.ply - 2].static_eval) {
        improvement = static_eval - td.stack[td.ply - 2].static_eval;
    }

    let improving = improvement > 0;

    // Razoring
    if !NODE::PV && !in_check && eval < alpha - 294 - 264 * depth * depth {
        return qsearch::<NonPV>(td, alpha, beta);
    }

    // Reverse Futility Pruning (RFP)
    if !tt_pv
        && !in_check
        && !excluded
        && depth <= 7
        && eval >= beta
        && eval
            >= beta + 72 * depth - (70 * improving as i32) - (23 * cut_node as i32)
                + 559 * correction_value.abs() / 1024
                + 23
        && !is_loss(beta)
        && !is_win(eval)
    {
        return (eval + beta) / 2;
    }

    // Null Move Pruning (NMP)
    if cut_node
        && !in_check
        && !excluded
        && eval >= beta
        && eval >= static_eval
        && static_eval >= beta - 15 * depth + 147 * tt_pv as i32 - 105 * improvement / 1024 + 187
        && td.ply as i32 >= td.nmp_min_ply
        && td.board.has_non_pawns()
        && !potential_singularity
        && !is_loss(beta)
    {
        let r = 5 + depth / 3 + ((eval - beta) / 244).min(3);

        td.stack[td.ply].conthist = std::ptr::null_mut();
        td.stack[td.ply].contcorrhist = std::ptr::null_mut();
        td.stack[td.ply].piece = Piece::None;
        td.stack[td.ply].mv = Move::NULL;
        td.ply += 1;

        td.board.make_null_move();

        let score = if (depth - r) <= 0 {
            -qsearch::<NonPV>(td, -beta, -beta + 1)
        } else {
            -search::<NonPV>(td, -beta, -beta + 1, depth - r, false)
        };

        td.board.undo_null_move();
        td.ply -= 1;

        if td.stopped {
            return Score::ZERO;
        }

        if score >= beta && !is_win(score) {
            if td.nmp_min_ply > 0 || depth < 16 {
                return score;
            }

            td.nmp_min_ply = td.ply as i32 + 3 * (depth - r) / 4;
            let verified_score = search::<NonPV>(td, beta - 1, beta, depth - r, false);
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
    let probcut_beta = beta + 271 - 61 * improving as i32;

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
                score = -search::<NonPV>(td, -probcut_beta, -probcut_beta + 1, probcut_depth, !cut_node);
            }

            undo_move(td, mv);

            if td.stopped {
                return Score::ZERO;
            }

            if score >= probcut_beta {
                td.tt.write(td.board.hash(), probcut_depth + 1, raw_eval, score, Bound::Lower, mv, td.ply, tt_pv);

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
            reduction += (494 - 425 * improvement / 128).min(1205);
        }

        if !NODE::ROOT && !is_loss(best_score) {
            let lmr_reduction = if is_quiet { reduction - 138 * history / 1024 } else { reduction };
            let lmr_depth = (depth - lmr_reduction / 1024).max(0);

            // Late Move Pruning (LMP)
            skip_quiets |= move_count >= (4 + depth * depth) / (2 - (improving || static_eval >= beta + 17) as i32);

            // Futility Pruning (FP)
            let futility_value = static_eval + 121 * lmr_depth + 76 + 35 * history / 1024;
            if !in_check
                && is_quiet
                && lmr_depth < 8
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
                + 114 * lmr_depth
                + 397 * move_count / 128
                + 81 * (history + 501) / 1024
                + 85 * PIECE_VALUES[td.board.piece_on(mv.to()).piece_type()] / 1024;

            if !in_check && lmr_depth < 6 && move_picker.stage() == Stage::BadNoisy && noisy_futility_value <= alpha {
                if !is_decisive(best_score) && best_score <= noisy_futility_value {
                    best_score = noisy_futility_value;
                }
                break;
            }

            // Static Exchange Evaluation Pruning (SEE Pruning)
            let threshold = if is_quiet {
                -22 * lmr_depth * lmr_depth - 44 * (history + 19) / 1024
            } else {
                -92 * depth + 45 - 43 * (history + 13) / 1024
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
                } else if score >= beta && !is_decisive(score) {
                    return score;
                } else if tt_score >= beta {
                    extension = -2;
                } else if cut_node {
                    extension = -2;
                }
            }
        }

        let initial_nodes = td.nodes.local();

        make_move(td, mv);

        let mut new_depth = depth + extension - 1;
        let mut score = Score::ZERO;

        // Late Move Reductions (LMR)
        if depth >= 3 && move_count > 1 + NODE::ROOT as i32 {
            if is_quiet {
                reduction -= 106 * (history - 574) / 1024;
            } else {
                reduction -= 95 * (history - 557) / 1024;
            }

            reduction -= 3268 * correction_value.abs() / 1024;
            reduction -= 55 * move_count;
            reduction += 303;

            if tt_pv {
                reduction -= 663;
                reduction -= 652 * (is_valid(tt_score) && tt_score > alpha) as i32;
                reduction -= 783 * (is_valid(tt_score) && tt_depth >= depth) as i32;
                reduction -= 796 * cut_node as i32;
            }

            if NODE::PV {
                reduction -= 590 + 573 * (beta - alpha > 34 * td.root_delta / 128) as i32;
            }

            if cut_node {
                reduction += 1193;
            }

            if td.board.in_check() || !td.board.has_non_pawns() {
                reduction -= 794;
            }

            if td.stack[td.ply].cutoff_count > 2 {
                reduction += 1232;
            }

            if is_valid(tt_score) && tt_score < alpha && tt_bound == Bound::Upper {
                reduction += 768;
            }

            let reduced_depth = (new_depth - reduction / 1024)
                .clamp(NODE::PV as i32, new_depth + cut_node as i32 + NODE::PV as i32)
                + NODE::PV as i32;

            td.stack[td.ply - 1].reduction = reduction;
            score = -search::<NonPV>(td, -alpha - 1, -alpha, reduced_depth, true);
            td.stack[td.ply - 1].reduction = 0;

            if score > alpha && new_depth > reduced_depth {
                new_depth += (score > best_score + 48 + 525 * depth / 128) as i32;
                new_depth -= (score < best_score + new_depth) as i32;

                if new_depth > reduced_depth {
                    score = -search::<NonPV>(td, -alpha - 1, -alpha, new_depth, !cut_node);

                    if mv.is_quiet() && score >= beta {
                        let bonus = (1 + 2 * (move_count > depth) as i32 + 2 * (move_count > 2 * depth) as i32)
                            * (162 * depth - 50).min(1037);
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
            td.node_table.add(mv, td.nodes.local() - initial_nodes);
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

                if depth > 2 && depth < 17 && !is_decisive(score) {
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
        let bonus_noisy = (128 * depth - 60).min(1150) - 69 * cut_node as i32;
        let malus_noisy = (145 * initial_depth - 67).min(1457) - 13 * (move_count - 1);

        let bonus_quiet = (151 * depth - 68).min(1597) - 64 * cut_node as i32;
        let malus_quiet = (134 * initial_depth - 55).min(1273) - 17 * (move_count - 1) + 200 * skip_quiets as i32;

        let bonus_cont = (97 * depth - 57).min(1250) - 69 * cut_node as i32;
        let malus_cont = (277 * initial_depth - 49).min(978) - 14 * (move_count - 1) + 126 * skip_quiets as i32;

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
            let mut factor = 107;
            factor += 141 * (initial_depth > 5) as i32;
            factor += 231 * (!in_check && best_score <= static_eval.min(raw_eval) - 135) as i32;
            factor += 289
                * (is_valid(td.stack[td.ply - 1].static_eval) && best_score <= -td.stack[td.ply - 1].static_eval - 102)
                    as i32;

            let scaled_bonus = factor * (148 * initial_depth - 43).min(1673) / 128;

            td.quiet_history.update(td.board.prior_threats(), !td.board.side_to_move(), pcm_move, scaled_bonus);

            for offset in [2, 3] {
                let bonus = (148 * initial_depth - 43).min(1673);

                if td.ply >= offset {
                    let entry = &td.stack[td.ply - offset];
                    if entry.mv.is_some() {
                        td.continuation_history.update(
                            entry.conthist,
                            td.stack[td.ply - 1].piece,
                            pcm_move.to(),
                            bonus,
                        );
                    }
                }
            }
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

    if alpha < Score::ZERO && td.board.upcoming_repetition(td.ply) {
        alpha = Score::ZERO;
        if alpha >= beta {
            return alpha;
        }
    }

    let in_check = td.board.in_check();

    if NODE::PV {
        td.pv.clear(td.ply);
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
            && (!NODE::PV || !is_decisive(tt_score))
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

        futility_score = static_eval + 123;
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
        }

        if !is_loss(best_score) && !td.board.see(mv, -73) {
            continue;
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

    let mut correction = td.pawn_corrhist.get(stm, td.board.pawn_key())
        + td.minor_corrhist.get(stm, td.board.minor_key())
        + td.major_corrhist.get(stm, td.board.major_key())
        + td.non_pawn_corrhist[Color::White].get(stm, td.board.non_pawn_key(Color::White))
        + td.non_pawn_corrhist[Color::Black].get(stm, td.board.non_pawn_key(Color::Black));

    if td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && td.stack[td.ply - 2].mv.is_some() {
        correction += td.continuation_corrhist.get(
            td.stack[td.ply - 2].contcorrhist,
            td.stack[td.ply - 1].piece,
            td.stack[td.ply - 1].mv.to(),
        );
    }

    correction
}

fn corrected_eval(eval: i32, correction_value: i32, hmr: u8) -> i32 {
    (eval * (200 - hmr as i32) / 200 + correction_value).clamp(-Score::TB_WIN_IN_MAX + 1, Score::TB_WIN_IN_MAX + 1)
}

fn update_correction_histories(td: &mut ThreadData, depth: i32, diff: i32) {
    let stm = td.board.side_to_move();
    let bonus = (138 * depth * diff / 128).clamp(-3964, 3303);

    td.pawn_corrhist.update(stm, td.board.pawn_key(), bonus);
    td.minor_corrhist.update(stm, td.board.minor_key(), bonus);
    td.major_corrhist.update(stm, td.board.major_key(), bonus);

    td.non_pawn_corrhist[Color::White].update(stm, td.board.non_pawn_key(Color::White), bonus);
    td.non_pawn_corrhist[Color::Black].update(stm, td.board.non_pawn_key(Color::Black), bonus);

    if td.ply >= 2 && td.stack[td.ply - 1].mv.is_some() && td.stack[td.ply - 2].mv.is_some() {
        td.continuation_corrhist.update(
            td.stack[td.ply - 2].contcorrhist,
            td.stack[td.ply - 1].piece,
            td.stack[td.ply - 1].mv.to(),
            bonus,
        );
    }
}

fn update_continuation_histories(td: &mut ThreadData, piece: Piece, sq: Square, bonus: i32) {
    for offset in [1, 2, 3, 4, 6] {
        if td.ply >= offset {
            let entry = &td.stack[td.ply - offset];
            if entry.mv.is_some() {
                td.continuation_history.update(entry.conthist, piece, sq, bonus);
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

    td.nodes.increment();
    td.nnue.push(mv, &td.board);
    td.board.make_move(mv);
    td.tt.prefetch(td.board.hash());
}

fn undo_move(td: &mut ThreadData, mv: Move) {
    td.ply -= 1;
    td.nnue.pop();
    td.board.undo_move(mv);
}
