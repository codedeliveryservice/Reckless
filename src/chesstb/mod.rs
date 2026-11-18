//! ChessTB tablebases: ChessTB's prober on Reckless's board, the root
//! rankers, and the table PV. Probes answer `None` where the tables can't
//! (a missing table or sub-table, or the derive depth cap).

mod cache;
mod common;
mod compress;
mod filesystem;
mod index;
mod layered;
mod memory;
mod probe;
mod root;
mod tables;

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering},
};

use self::{
    common::{IGNORE_50MR, WdlEntry},
    probe::{probe_dtm_impl, probe_wdl_impl, wdl_rank},
    root::{Options, Ranked},
};
use crate::{board::Board, thread::ThreadData, types::Move};

static MAX_CARDINALITY: AtomicUsize = AtomicUsize::new(0);
static PATH: Mutex<String> = Mutex::new(String::new());

// UCI options.
static PROBE_DEPTH: AtomicI32 = AtomicI32::new(1);
static MATE_PROBE_DEPTH: AtomicI32 = AtomicI32::new(20);
static RULE50: AtomicBool = AtomicBool::new(true);
static PROBE_LIMIT: AtomicUsize = AtomicUsize::new(8);
static MOVE_OVERHEAD: AtomicU64 = AtomicU64::new(100);

// The current search's settings, from the root ranking.
static SEARCH_CARDINALITY: AtomicUsize = AtomicUsize::new(0);
static SEARCH_PROBE_DEPTH: AtomicI32 = AtomicI32::new(0);
static SEARCH_MATE_PROBE_DEPTH: AtomicI32 = AtomicI32::new(0);
static SEARCH_RULE50: AtomicBool = AtomicBool::new(true);

pub fn print_options() {
    println!("option name ChessTBPath type string default <empty>");
    println!("option name ChessTBProbeDepth type spin default 1 min 1 max 100");
    println!("option name ChessTBMateProbeDepth type spin default 20 min 1 max 100");
    println!("option name ChessTB50MoveRule type check default true");
    println!("option name ChessTBProbeLimit type spin default 8 min 0 max 8");
    println!("option name ChessTBCache type spin default 64 min 1 max 1048576");
}

/// Sets a ChessTB option; false if `name` is none or `value` doesn't parse.
pub fn set_option(name: &str, value: &str) -> bool {
    let spin = |min: i64, max: i64| value.parse::<i64>().ok().map(|v| v.clamp(min, max));
    match name {
        "ChessTBPath" => initialize_and_report(value),
        "ChessTBProbeDepth" => match spin(1, 100) {
            Some(v) => PROBE_DEPTH.store(v as i32, Ordering::Relaxed),
            None => return false,
        },
        "ChessTBMateProbeDepth" => match spin(1, 100) {
            Some(v) => MATE_PROBE_DEPTH.store(v as i32, Ordering::Relaxed),
            None => return false,
        },
        "ChessTB50MoveRule" => match value.parse::<bool>() {
            Ok(v) => RULE50.store(v, Ordering::Relaxed),
            Err(_) => return false,
        },
        "ChessTBProbeLimit" => match spin(0, 8) {
            Some(v) => PROBE_LIMIT.store(v as usize, Ordering::Relaxed),
            None => return false,
        },
        "ChessTBCache" => match spin(1, 1 << 20) {
            Some(v) => probe::BLOCKS.set_max_bytes((v as usize) << 20),
            None => return false,
        },
        _ => return false,
    }
    true
}

/// The move overhead in ms, which bounds the PV extension.
pub fn set_move_overhead(ms: u64) {
    MOVE_OVERHEAD.store(ms, Ordering::Relaxed);
}

/// Opens the tables under `paths` (ChessTBPath), dropping the open ones;
/// returns the men of the largest table found.
pub fn initialize(paths: &str) -> usize {
    let size = probe::init(paths);
    MAX_CARDINALITY.store(size, Ordering::Relaxed);
    *PATH.lock().unwrap() = paths.to_string();
    root::reset_walk();
    size
}

/// `initialize`, telling the GUI the largest table's men.
pub fn initialize_and_report(paths: &str) {
    let size = initialize(paths);
    println!("info string Found ChessTB tables of up to {size} pieces");
}

/// Drops the open tables and opens them again.
pub fn reinitialize() {
    let path = PATH.lock().unwrap().clone();
    initialize(&path);
}

pub fn max_cardinality() -> usize {
    MAX_CARDINALITY.load(Ordering::Relaxed)
}

fn options() -> Options {
    Options {
        probe_depth: PROBE_DEPTH.load(Ordering::Relaxed),
        mate_probe_depth: MATE_PROBE_DEPTH.load(Ordering::Relaxed),
        rule50: RULE50.load(Ordering::Relaxed),
        probe_limit: PROBE_LIMIT.load(Ordering::Relaxed),
        move_overhead: MOVE_OVERHEAD.load(Ordering::Relaxed),
    }
}

/// WDL from the side to move, -2 (loss) ..= 2 (win), 1 and -1 cursed.
pub fn probe_wdl(board: &mut Board) -> Option<i32> {
    let wdl = probe_wdl_impl(board, 0);
    (wdl != WdlEntry::Illegal).then(|| wdl_rank(wdl) - 2)
}

/// DTM, and DTM50 at the position's clock (`rule50`): plies to mate, positive
/// when the side to move mates; 0 for a draw or no answer. A distance read:
/// search gates it.
pub fn probe_dtm(board: &mut Board, rule50: bool) -> (i32, i32) {
    let clock = if rule50 { board.fiftymove_clock() as u32 } else { IGNORE_50MR };
    let r = probe_dtm_impl(board, clock, 0);
    let read = |clocked: bool| {
        let has = if clocked { r.has_dtm50 } else { r.has_dtm };
        if !has {
            return 0;
        }
        let (w, d) = if clocked { (r.dtm50_wdl, r.dtm50) } else { (probe::fold_dtm_wdl(r.wdl), r.dtm) };
        match w {
            WdlEntry::Win => d as i32,
            WdlEntry::Lose => -(d as i32),
            _ => 0,
        }
    };
    (read(false), read(true))
}

/// Search's probing settings for the current search.
#[derive(Copy, Clone)]
pub struct SearchConfig {
    pub mate_probe_depth: i32,
    pub use_rule50: bool,
}

/// Whether search probes a node of `pieces` men at `depth`.
pub fn probes_at(pieces: usize, depth: i32) -> bool {
    let cardinality = SEARCH_CARDINALITY.load(Ordering::Relaxed);
    pieces <= cardinality && (pieces < cardinality || depth >= SEARCH_PROBE_DEPTH.load(Ordering::Relaxed))
}

pub fn search_config() -> SearchConfig {
    SearchConfig {
        mate_probe_depth: SEARCH_MATE_PROBE_DEPTH.load(Ordering::Relaxed),
        use_rule50: SEARCH_RULE50.load(Ordering::Relaxed),
    }
}

/// Ranks the root moves of the main thread's board where it is in the tables,
/// sorting them by rank, and sets search's probing for the search.
pub fn rank_root_moves(td: &mut ThreadData) {
    let mut moves: Vec<Ranked> = td.root_moves.iter().map(|rm| Ranked::new(rm.mv)).collect();
    let config = root::rank_root_moves(&mut td.board, &mut moves, &options(), max_cardinality());

    for rm in &mut td.root_moves {
        let ranked = moves.iter().find(|r| r.mv == rm.mv);
        rm.tb_rank = if config.root_in_tb { ranked.map_or(0, |r| r.tb_rank) } else { 0 };
        rm.tb_score = if config.root_in_tb { ranked.map_or(0, |r| r.tb_score) } else { 0 };
    }
    // A stable sort from the order the ranker started from, as the ranker's
    // own: moves of equal rank keep their relative order in both.
    td.root_moves.sort_by_key(|rm| std::cmp::Reverse(rm.tb_rank));

    SEARCH_CARDINALITY.store(config.cardinality, Ordering::Relaxed);
    SEARCH_PROBE_DEPTH.store(config.probe_depth, Ordering::Relaxed);
    SEARCH_MATE_PROBE_DEPTH.store(config.mate_probe_depth, Ordering::Relaxed);
    SEARCH_RULE50.store(config.use_rule50, Ordering::Relaxed);
    td.shared.root_in_tb.store(config.root_in_tb, Ordering::Relaxed);
    td.shared.stop_probing_tb.store(config.cardinality == 0, Ordering::Relaxed);
}

/// Corrects and extends a root move's PV (`pv`, the root move first) with a
/// TB score `v`, within the move overhead: where the PV is in the tables,
/// their line toward mate. May make `v` exact (a proven mate) or,
/// exceptionally, a draw.
pub fn extend_pv(board: &mut Board, pv: &mut Vec<Move>, v: &mut i32, multi_pv: usize) {
    root::extend_pv(board, pv, v, multi_pv, &options(), max_cardinality());
}
