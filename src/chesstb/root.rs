//! The root rankers (DTM50 or DTM, else DTZ with DTC re-ranking, else WDL)
//! and the table PV.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use super::{common::*, probe::*};
use crate::{
    board::{Board, NullBoardObserver},
    types::{MAX_PLY, Move, PieceType, Score, mate_in, mated_in},
};

pub const MAX_DTZ: i32 = 1 << 18;

/// A pawn in endgame eval units (`normalize_to_cp` at its least material):
/// the scale of the root scores of wins and losses the clock draws, so the
/// least of them still prints as a centipawn.
const PAWN_VALUE: i32 = 382;

/// A root move and the tables' rank and score for it.
#[derive(Copy, Clone)]
pub struct Ranked {
    pub mv: Move,
    pub tb_rank: i32,
    pub tb_score: i32,
}

impl Ranked {
    pub const fn new(mv: Move) -> Self {
        Self { mv, tb_rank: 0, tb_score: 0 }
    }
}

/// The ranking's search settings: probe in search up to `cardinality` men
/// (0: not at all).
#[derive(Copy, Clone)]
pub struct Config {
    pub cardinality: usize,
    pub root_in_tb: bool,
    pub use_rule50: bool,
    pub probe_depth: i32,
    pub mate_probe_depth: i32,
}

/// The engine's options the rankers read.
#[derive(Copy, Clone)]
pub struct Options {
    pub probe_depth: i32,
    pub mate_probe_depth: i32,
    pub rule50: bool,
    pub probe_limit: usize,
    pub move_overhead: u64,
}

const WDL_TO_VALUE: [i32; 5] = [-Score::TB_WIN, -2, 0, 2, Score::TB_WIN];

/// The 50-move rule or a repetition draws the position (Stockfish's
/// `is_draw`, without Reckless's material draws).
pub fn is_draw(board: &Board, ply: i32) -> bool {
    board.draw_by_fifty_move_rule() || board.draw_by_repetition(ply)
}

/// Pawnless with 3 men, or 4 with neither queen nor rook: checkmate is the
/// only zeroing move that matters.
fn dtz_is_dtm(board: &Board) -> bool {
    let n = board.occupancies().popcount();
    board.pieces(PieceType::Pawn).is_empty()
        && (n == 3 || (n == 4 && board.pieces2(PieceType::Queen, PieceType::Rook).is_empty()))
}

/// The engine's metric in a probe (DTM50 under the 50-move rule, DTM without
/// it): Win, Loss or Draw, and the plies to mate.
fn mate_of(r: &ProbeResult, rule50: bool) -> Option<(WdlEntry, i32)> {
    let has = if rule50 { r.has_dtm50 } else { r.has_dtm };
    if !has {
        return None;
    }
    let wdl = if rule50 { r.dtm50_wdl } else { fold_dtm_wdl(r.wdl) };
    let plies = if wdl == WdlEntry::Draw {
        0
    } else if rule50 {
        r.dtm50 as i32
    } else {
        r.dtm as i32
    };
    Some((wdl, plies))
}

const fn probe_clock(board: &Board, rule50: bool) -> u32 {
    if rule50 { board.fiftymove_clock() as u32 } else { IGNORE_50MR }
}

/// The child's answer after `m`. A draw by repetition at the child is a known
/// draw in either metric.
fn probe_mate_after_move(board: &mut Board, m: Move, rule50: bool) -> ProbeResult {
    board.make_move(m, &mut NullBoardObserver);
    let r = if !board.draw_by_repetition(1) && !is_bare_kings(board) {
        probe_dtm_impl(board, probe_clock(board, rule50), 0)
    } else {
        ProbeResult {
            wdl: WdlEntry::Draw,
            has_dtm: true,
            dtm50_wdl: WdlEntry::Draw,
            has_dtm50: true,
            ..Default::default()
        }
    };
    board.undo_move(m);
    r
}

/// A move's distance to mate from the mover's side: the child's, one ply more.
fn move_mate_of(child: &ProbeResult, rule50: bool) -> Option<(WdlEntry, i32)> {
    let (child_wdl, child_plies) = mate_of(child, rule50)?;
    Some((invert_wdl(child_wdl), if child_wdl == WdlEntry::Draw { 0 } else { child_plies + 1 }))
}

const fn signed_dtz(w: WdlEntry, dtz: i32) -> i32 {
    match w {
        WdlEntry::Win | WdlEntry::CursedWin => dtz,
        WdlEntry::BlessedLoss | WdlEntry::Lose => -dtz,
        _ => 0,
    }
}

const fn zeroing_signed_dtz(my_wdl: WdlEntry) -> i32 {
    match my_wdl {
        WdlEntry::Win => 1,
        WdlEntry::CursedWin => 101,
        WdlEntry::BlessedLoss => -101,
        WdlEntry::Lose => -1,
        _ => 0,
    }
}

fn mate_value(wdl: WdlEntry, plies: i32) -> i32 {
    if wdl == WdlEntry::Draw {
        return 0;
    }
    let v = if plies < MAX_PLY as i32 { Score::MATE - plies } else { Score::TB_WIN };
    if wdl == WdlEntry::Win { v } else { -v }
}

/// Ranks root moves by DTZ, as ChessTB's probe_root_dtz. DTZ is exact but for
/// a cursed one, which may read a ply short: only an order among moves that
/// rank after every clean one.
fn root_probe(
    board: &mut Board, moves: &mut [Ranked], rule50: bool, rank_distance: bool, time_abort: &dyn Fn() -> bool,
) -> bool {
    // The clock counts with the rule off too, as in Syzygy, so play keeps
    // converting; only the score bound depends on the rule.
    let cnt50 = board.fiftymove_clock() as i32;
    let rep = board.has_repeated();
    let bound = if rule50 { MAX_DTZ / 2 - 100 } else { 1 };
    let rights = castling_count(board);

    // Each move's DTZ from the root (positive wins), and DTC's pawn pushes
    // owed and plies to the next zeroing move, where it has them.
    #[derive(Copy, Clone)]
    struct MoveDtz {
        dtz: i32,
        mine: WdlEntry,
        order: i32,
        wait: i32,
    }
    let mut dtzs = vec![MoveDtz { dtz: 0, mine: WdlEntry::Draw, order: -1, wait: 0 }; moves.len()];

    for (k, md) in dtzs.iter_mut().enumerate() {
        let mv = moves[k].mv;
        let zeroes = is_zeroing(board, mv);
        board.make_move(mv, &mut NullBoardObserver);

        let mut ok = true;
        if zeroes {
            let child_wdl = if is_bare_kings(board) { WdlEntry::Draw } else { probe_wdl_impl(board, 0) };
            ok = child_wdl != WdlEntry::Illegal;
            md.mine = invert_wdl(child_wdl);
            md.dtz = zeroing_signed_dtz(md.mine);
        } else if board.draw_by_repetition(1) {
            md.dtz = 0;
        } else {
            let mut cd = Some((0u16, false));
            let mut cw = WdlEntry::Draw;
            if !is_bare_kings(board) {
                let cr = probe_dtc_impl(board, IGNORE_50MR, 0);
                cw = cr.wdl;
                cd = cr.has_dtz.then_some((cr.dtz, cr.dtz_rounded));
            }
            ok = cw != WdlEntry::Illegal && cd.is_some();
            md.mine = invert_wdl(cw);
            let (cdtz, crounded) = cd.unwrap_or_default();
            let child = if ok { -signed_dtz(cw, cdtz as i32) } else { 0 };
            if ok && cw == WdlEntry::Lose && cdtz == 0 {
                md.dtz = 1; // mate
            } else if child != 0 {
                let exit = is_cursed_exit(rights, board, cw);
                let dtz = quiet_move_dtz(child.unsigned_abs() as u16, exit, crounded) as i32;
                let cursed = matches!(cw, WdlEntry::CursedWin | WdlEntry::BlessedLoss) || dtz > 100;
                // Cursed moves rank after every clean one, by their distance.
                md.dtz = child.signum() * if cursed { 100 + dtz } else { dtz };
                if dtz > DTZ_MAX_NON_CURSED as i32 {
                    md.mine = match md.mine {
                        WdlEntry::Win => WdlEntry::CursedWin,
                        WdlEntry::Lose => WdlEntry::BlessedLoss,
                        w => w,
                    };
                }
            }
        }
        if rank_distance && ok && md.dtz != 0 {
            let conversion = is_conversion(mv);
            // A push to mate is preferred at any budget.
            let mate = is_checkmate(board);
            if (conversion || mate) && is_decisive_class(md.mine) {
                md.order = 0;
                md.wait = 1;
            } else if !conversion {
                let child_rule50 = if zeroes {
                    0
                } else if rule50 {
                    cnt50 as u32 + 1
                } else {
                    1
                };
                let cr = probe_dtc_impl(board, child_rule50, 0);
                if cr.has_dtc && is_decisive_class(cr.dtc_wdl) {
                    let spends = zeroes && md.mine == WdlEntry::Win;
                    md.order = cr.dtc_order as i32 + spends as i32;
                    md.wait = if zeroes { 1 } else { cr.dtc as i32 + 1 };
                }
            }
        }
        board.undo_move(mv);

        if time_abort() || !ok {
            return false;
        }
    }

    for (m, md) in moves.iter_mut().zip(&dtzs) {
        let dtz = md.dtz;
        // Ranking by distance: by DTC's pushes owed, then plies (DTC's, else
        // DTZ). ChessTB sorts by the pair; here one key (plies <= 100), as
        // tb_rank is search's group.
        let order = md.order.max(0);
        let plies = if md.order >= 0 { md.wait } else { dtz.abs() };
        let key = if rank_distance { order * 128 + plies } else { 0 };
        // A win is certain while it wins at this clock, a loss unless a
        // 50-move draw is in sight. DTZ 1 (zeroing or mate) decides at any
        // root clock, repeated or not, as a push to mate is preferred at any
        // budget.
        let got = if dtz > 0 {
            if dtz == 1 || (dtz + cnt50 <= 100 && !rep) { MAX_DTZ - key } else { MAX_DTZ / 2 - (dtz + cnt50) }
        } else if dtz < 0 {
            if dtz == -1 || -dtz * 2 + cnt50 < 100 { -MAX_DTZ + key } else { -MAX_DTZ / 2 + (-dtz + cnt50) }
        } else {
            0
        };
        m.tb_rank = got;
        m.tb_score = if got >= bound {
            Score::TB_WIN
        } else if got > 0 {
            (3.max(got - (MAX_DTZ / 2 - 200)) * PAWN_VALUE) / 200
        } else if got == 0 {
            0
        } else if got > -bound {
            ((-3).min(got + (MAX_DTZ / 2 - 200)) * PAWN_VALUE) / 200
        } else {
            -Score::TB_WIN
        };
    }
    true
}

/// Ranks root moves by DTM50 (50-move rule) or DTM, each distance its own
/// rank. False when a child has no answer.
fn root_probe_mate(board: &mut Board, moves: &mut [Ranked], rule50: bool) -> bool {
    for m in moves.iter_mut() {
        let cr = probe_mate_after_move(board, m.mv, rule50);
        let Some((wdl, plies)) = move_mate_of(&cr, rule50) else { return false };
        // No root clock check: as in search, each child stands at its own
        // clock.
        m.tb_rank = match wdl {
            WdlEntry::Win => MAX_DTZ - plies,
            WdlEntry::Lose => -MAX_DTZ + plies,
            _ => 0,
        };
        m.tb_score = mate_value(wdl, plies);

        // A win the clock draws stays just above a draw, ranked by DTM; such
        // a loss just below.
        if wdl == WdlEntry::Draw && rule50 {
            let cls = fold_dtm_wdl(invert_wdl(cr.wdl));
            if cls == WdlEntry::Draw {
                continue;
            }
            if !cr.has_dtm {
                return false;
            }
            let flat = cr.dtm as i32 + 1;
            let win = cls == WdlEntry::Win;
            m.tb_rank = if win { 1.max(MAX_DTZ / 2 - flat) } else { (-1).min(flat - MAX_DTZ / 2) };
            m.tb_score = if win { 2 } else { -2 };
        }
    }
    true
}

fn root_probe_wdl(board: &mut Board, moves: &mut [Ranked], rule50: bool) -> bool {
    const WDL_TO_RANK: [i32; 5] = [-MAX_DTZ, -MAX_DTZ + 101, 0, MAX_DTZ - 101, MAX_DTZ];

    for m in moves.iter_mut() {
        board.make_move(m.mv, &mut NullBoardObserver);
        // With WDL alone a quiet win at clock 100 would rank as a win.
        let mut cls = WdlEntry::Draw;
        if (!rule50 || !is_draw(board, 1)) && !board.draw_by_repetition(1) {
            cls = if is_bare_kings(board) { WdlEntry::Draw } else { invert_wdl(probe_wdl_impl(board, 0)) };
        }
        board.undo_move(m.mv);
        if cls == WdlEntry::Illegal {
            return false;
        }

        if !rule50 {
            cls = fold_dtm_wdl(cls);
        }
        m.tb_rank = WDL_TO_RANK[wdl_rank(cls) as usize];
        m.tb_score = WDL_TO_VALUE[wdl_rank(cls) as usize];
    }
    true
}

/// Ranks the root moves where the root is in the tables (`root_in_tb`), and
/// sorts them by rank; else zeroes their ranks. Exact distance to mate where
/// there is one, else DTZ, else WDL.
pub fn rank_root_moves(board: &mut Board, moves: &mut [Ranked], opts: &Options, max_cardinality: usize) -> Config {
    let mut config = Config {
        cardinality: 0,
        root_in_tb: false,
        use_rule50: opts.rule50,
        probe_depth: opts.probe_depth,
        mate_probe_depth: opts.mate_probe_depth,
    };
    if moves.is_empty() {
        return config;
    }
    config.cardinality = opts.probe_limit;

    // Tables with fewer pieces than the probe limit are searched at any depth.
    if config.cardinality > max_cardinality {
        config.cardinality = max_cardinality;
        config.probe_depth = 0;
    }

    let mut dtz_available = true;
    if config.cardinality >= board.occupancies().popcount() {
        // Rank by distance if checkmate is the only zeroing move.
        let rank_distance = dtz_is_dtm(board);

        config.root_in_tb = root_probe_mate(board, moves, config.use_rule50)
            || root_probe(board, moves, config.use_rule50, rank_distance, &|| false);
        if !config.root_in_tb {
            // DTZ is missing: rank by WDL.
            dtz_available = false;
            config.root_in_tb = root_probe_wdl(board, moves, config.use_rule50);
        }
    }

    if config.root_in_tb {
        moves.sort_by_key(|m| std::cmp::Reverse(m.tb_rank));
        // Probe during search only if DTZ is not available and we are winning.
        if dtz_available || moves[0].tb_score <= 0 {
            config.cardinality = 0;
        }
    } else {
        for m in moves.iter_mut() {
            m.tb_rank = 0;
        }
    }
    config
}

/// Every DTM and DTM50 is at most 2045 plies (ChessTB's CORRECTNESS.md, RANGE).
const MAX_LINE: usize = 2048;

/// The PV extension's walk so far, kept between outputs; `init` drops it.
struct Walk {
    history: u64,
    rule50: bool,
    seek_mate: bool, // whether the line takes distance steps only
    clock: u8,
    line: Vec<Move>,
    known: bool, // the distance at the line's end
    wdl: WdlEntry,
    plies: i32,
}

impl Walk {
    const fn new(history: u64, rule50: bool, seek_mate: bool, clock: u8) -> Self {
        Self {
            history,
            rule50,
            seek_mate,
            clock,
            line: Vec::new(),
            known: false,
            wdl: WdlEntry::Draw,
            plies: 0,
        }
    }
}

static WALK: Mutex<Walk> = Mutex::new(Walk::new(0, false, false, 0));

pub fn reset_walk() {
    *WALK.lock().unwrap() = Walk::new(0, false, false, 0);
}

/// From the PV's first position in the tables, the tables' line until mate, a
/// draw or the deadline (a share of the move overhead: a read can decode a
/// block). `v` becomes the mate when the line proves one, or a draw when the
/// clock draws the root move.
pub fn extend_pv(
    board: &mut Board, pv: &mut Vec<Move>, v: &mut i32, multi_pv: usize, opts: &Options, max_cardinality: usize,
) {
    if pv.is_empty() {
        return;
    }
    let deadline = Instant::now() + Duration::from_micros(1000 * opts.move_overhead / multi_pv.max(1) as u64);
    let rule50 = opts.rule50;
    let seek_mate = v.abs() >= Score::MATE_IN_MAX;
    let late = move || Instant::now() >= deadline;
    // Search's line up to its first position within the probe limit.
    let limit = opts.probe_limit.min(max_cardinality);
    let searched = pv.clone();
    let mut k = 0;
    while k < searched.len() && (k == 0 || board.occupancies().popcount() > limit) {
        board.make_move(searched[k], &mut NullBoardObserver);
        k += 1;
    }
    if board.occupancies().popcount() > limit || late() {
        for &m in searched[..k].iter().rev() {
            board.undo_move(m);
        }
        return;
    }
    pv.truncate(k);

    // The line depends on the keys a repetition check sees: an ordered digest
    // of them, as trustworthy as the 64-bit keys the checks compare.
    let history = board.history_digest();

    // The line proves a mate only if the distance probe right after the root
    // move succeeded.
    let first = mate_of(&probe_dtm_impl(board, probe_clock(board, rule50), 0), rule50);
    let (mut known, mut wdl, mut plies) = match first {
        Some((w, p)) => (true, w, p),
        None => (false, WdlEntry::Draw, 0),
    };
    let exact = k == 1 && known;
    // A draw at the clock there outranks the root's TB score: a WDL-ranked
    // root can't see the clock.
    let drawn = exact && wdl == WdlEntry::Draw;

    let mut walk = WALK.lock().unwrap();

    // Outputs mostly enter at the same position: continue the kept walk. Its
    // moves are checked as TT moves are: a digest collision must not replay
    // an illegal one.
    let clock = board.fiftymove_clock();
    let mut kept =
        walk.history == history && walk.rule50 == rule50 && walk.seek_mate == seek_mate && walk.clock == clock;
    if kept {
        for &m in &walk.line {
            if !m.is_present() || !board.is_legal(m) {
                for &done in pv[k..].iter().rev() {
                    board.undo_move(done);
                }
                pv.truncate(k);
                kept = false;
                break;
            }
            board.make_move(m, &mut NullBoardObserver);
            pv.push(m);
        }
    }
    if kept {
        (known, wdl, plies) = (walk.known, walk.wdl, walk.plies);
    } else {
        *walk = Walk::new(history, rule50, seek_mate, clock);
    }

    // Each ply, play the first move that is one ply closer to mate. Where the
    // tables have no distance, play the root ranking's top move instead,
    // unless only a mate is sought.
    while walk.line.len() < MAX_LINE && !late() {
        let mut next = None;
        if known {
            if wdl == WdlEntry::Draw || plies <= 0 {
                break;
            }
            for entry in legal_moves(board).iter() {
                if late() {
                    break;
                }
                let m = entry.mv;
                if move_mate_of(&probe_mate_after_move(board, m, rule50), rule50) == Some((wdl, plies)) {
                    next = Some(m);
                    break;
                }
            }
        } else if !seek_mate && (!rule50 || !is_draw(board, 0)) {
            let mut moves: Vec<Ranked> = legal_moves(board).iter().map(|e| Ranked::new(e.mv)).collect();
            if moves.is_empty() || !root_probe(board, &mut moves, rule50, true, &late) {
                break;
            }
            // The first of the best.
            let top = moves.iter().fold(moves[0], |top, m| if m.tb_rank > top.tb_rank { *m } else { top });
            if top.tb_rank != 0 {
                next = Some(top.mv);
            }
        }
        let Some(next) = next else { break };
        board.make_move(next, &mut NullBoardObserver);
        pv.push(next);
        walk.line.push(next);
        if known {
            wdl = invert_wdl(wdl);
            plies -= 1;
        } else if let Some((w, p)) = mate_of(&probe_dtm_impl(board, probe_clock(board, rule50), 0), rule50) {
            (known, wdl, plies) = (true, w, p);
        }
    }
    (walk.known, walk.wdl, walk.plies) = (known, wdl, plies);
    drop(walk);

    let mated = is_checkmate(board);
    for &m in pv.iter().rev() {
        board.undo_move(m);
    }
    if pv.len() == k {
        // No table move: search's line stays.
        *pv = searched;
    }
    if drawn {
        *v = 0;
        return;
    }
    let length = pv.len();
    if !mated || !exact || length >= MAX_PLY {
        // Past MAX_PLY no mate score.
        return;
    }
    // Odd: the root's side gave mate.
    *v = if length % 2 == 1 { mate_in(length as isize) } else { mated_in(length as isize) };
}
