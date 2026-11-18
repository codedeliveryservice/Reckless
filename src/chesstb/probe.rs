//! The prober on a Reckless board: table routing and the open tables, then
//! each metric's read, deriving what a table doesn't store by one ply of
//! search. A probe answers `WdlEntry::Illegal` where the tables can't.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::LocalKey,
};

use super::{
    cache::{BlockPool, next_epoch},
    common::*,
    filesystem::file_exists_case_exact,
    index::{PairGroup, PieceConfig},
    layered::{
        DTC_BUDGET_LAYERS, DTC_PACK_LAYERS, DtcCell, DtcCurve, DtcFile, Dtm50Cell, Dtm50File, dtc_budget_plies,
        read_dtc, read_dtc_curve, read_dtm50,
    },
    tables::*,
};
use crate::{
    board::{Board, NullBoardObserver},
    types::{Bitboard, Color, Move, MoveList, Piece, PieceType, Square},
};

const DEFAULT_CACHE_MB: usize = 64;

/// Changes whenever `init` drops the open tables: a thread's cached table
/// handle is valid only while its epoch is the current one.
static EPOCH: AtomicU64 = AtomicU64::new(0);
/// Decoded blocks of every open table, within ChessTBCache.
pub static BLOCKS: LazyLock<Arc<BlockPool>> = LazyLock::new(|| Arc::new(BlockPool::new(DEFAULT_CACHE_MB << 20)));

type Handle<F> = Option<Arc<F>>;

#[derive(Default)]
struct Tables {
    dirs: [Vec<PathBuf>; 5],
    wdl: HashMap<MaterialKey, Handle<WdlFile>>,
    dtz: HashMap<MaterialKey, Handle<DtzFile>>,
    dtc: HashMap<MaterialKey, Handle<DtcFile>>,
    dtm: HashMap<MaterialKey, Handle<DtmFile>>,
    dtm50: HashMap<MaterialKey, Handle<Dtm50File>>,
}

static TABLES: LazyLock<Mutex<Tables>> = LazyLock::new(|| Mutex::new(Tables::default()));

const WDL: usize = 0;
const DTZ: usize = 1;
const DTC: usize = 2;
const DTM: usize = 3;
const DTM50: usize = 4;
const METRIC_EXT: [&str; 5] = [WDL_EXT, DTZ_EXT, DTC_EXT, DTM_EXT, DTM50_EXT];
const METRIC_MAGIC: [u32; 5] = [WDL_MAGIC, DTZ_MAGIC, DTC_MAGIC, DTM_MAGIC, DTM50_MAGIC];
const METRIC_SUBDIR: [&str; 5] = ["wdl", "dtz", "dtc", "dtm", "dtm50"];

/// A thread's last opened tables of one metric, by epoch and material.
struct TlCache<F> {
    epoch: [u64; 16],
    key: [MaterialKey; 16],
    val: [Handle<F>; 16],
    rr: usize,
}

impl<F> Default for TlCache<F> {
    fn default() -> Self {
        Self {
            epoch: [0; 16],
            key: [MaterialKey::default(); 16],
            val: std::array::from_fn(|_| None),
            rr: 0,
        }
    }
}

type TlTables<F> = LocalKey<RefCell<TlCache<F>>>;

thread_local! {
    static TL_WDL: RefCell<TlCache<WdlFile>> = RefCell::default();
    static TL_DTZ: RefCell<TlCache<DtzFile>> = RefCell::default();
    static TL_DTC: RefCell<TlCache<DtcFile>> = RefCell::default();
    static TL_DTM: RefCell<TlCache<DtmFile>> = RefCell::default();
    static TL_DTM50: RefCell<TlCache<Dtm50File>> = RefCell::default();
}

/// The table for `ps`, opened once per epoch: from the thread's own cache,
/// else the shared one, else from disk.
fn open_table<P: PerColor>(
    metric: usize, ps: &PieceConfig, tl: &'static TlTables<TableFile<P>>,
    cache: fn(&mut Tables) -> &mut HashMap<MaterialKey, Handle<TableFile<P>>>,
) -> Handle<TableFile<P>> {
    let k = ps.min_material_key();
    loop {
        let e = EPOCH.load(Ordering::Acquire);

        let hit = tl.with_borrow(|t| {
            (1..=16)
                .map(|n| t.rr.wrapping_sub(n) & 15)
                .find(|&i| t.epoch[i] == e && t.key[i] == k)
                .map(|i| t.val[i].clone())
        });
        if let Some(h) = hit {
            return h;
        }
        let insert = |h: &Handle<TableFile<P>>| {
            tl.with_borrow_mut(|t| {
                let i = t.rr & 15;
                t.rr = t.rr.wrapping_add(1);
                t.epoch[i] = e;
                t.key[i] = k;
                t.val[i] = h.clone();
            })
        };

        let dirs = {
            let mut tables = TABLES.lock().unwrap();
            if let Some(h) = cache(&mut tables).get(&k) {
                let h = h.clone();
                drop(tables);
                insert(&h);
                return h;
            }
            tables.dirs[metric].clone()
        };

        let name = ps.name() + METRIC_EXT[metric];
        let built = dirs
            .iter()
            .map(|d| d.join(&name))
            .find(|p| file_exists_case_exact(p))
            .and_then(|p| TableFile::<P>::load(ps, &p, METRIC_MAGIC[metric], &BLOCKS))
            .map(Arc::new);

        let mut tables = TABLES.lock().unwrap();
        if EPOCH.load(Ordering::Relaxed) != e {
            continue;
        }
        let h = cache(&mut tables).entry(k).or_insert(built).clone();
        drop(tables);
        insert(&h);
        return h;
    }
}

fn open_wdl(ps: &PieceConfig) -> Handle<WdlFile> {
    open_table(WDL, ps, &TL_WDL, |t| &mut t.wdl)
}

fn open_dtz(ps: &PieceConfig) -> Handle<DtzFile> {
    open_table(DTZ, ps, &TL_DTZ, |t| &mut t.dtz)
}

fn open_dtc(ps: &PieceConfig) -> Handle<DtcFile> {
    open_table(DTC, ps, &TL_DTC, |t| &mut t.dtc)
}

fn open_dtm(ps: &PieceConfig) -> Handle<DtmFile> {
    open_table(DTM, ps, &TL_DTM, |t| &mut t.dtm)
}

fn open_dtm50(ps: &PieceConfig) -> Handle<Dtm50File> {
    open_table(DTM50, ps, &TL_DTM50, |t| &mut t.dtm50)
}

/// Pieces in the largest table found, kings included.
fn scan_paths(tables: &Tables) -> usize {
    let mut lg = 0;
    for (metric, dirs) in tables.dirs.iter().enumerate() {
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for e in entries.flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.ends_with(METRIC_EXT[metric]) {
                    lg = lg.max(n.find('.').unwrap_or(n.len()));
                }
            }
        }
    }
    lg
}

fn add_search_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dirs.iter().any(|d| d.components().eq(dir.components())) {
        dirs.push(dir);
    }
}

/// One directory: each metric's subdirectory, or the directory itself.
fn init_dir(tables: &mut Tables, dir: &Path) {
    for (metric, sub) in METRIC_SUBDIR.iter().enumerate() {
        let p = dir.join(sub);
        add_search_dir(&mut tables.dirs[metric], if p.is_dir() { p } else { dir.to_path_buf() });
    }
}

/// Directories as ChessTBPath, ':'-separated (';' on Windows); each metric in
/// its wdl/ ... dtm50/ subdirectory or the directory itself, and so its
/// castling/. Drops every open table; returns the largest table's men.
pub fn init(paths: &str) -> usize {
    let mut tables = TABLES.lock().unwrap();
    *tables = Tables::default();
    EPOCH.store(next_epoch(), Ordering::Release);

    let sep = if cfg!(windows) { ';' } else { ':' };
    for path in paths.split(sep) {
        if path.is_empty() || path == "<empty>" {
            continue;
        }
        let dir = Path::new(path);
        init_dir(&mut tables, dir);
        if dir.join("castling").is_dir() {
            init_dir(&mut tables, &dir.join("castling"));
        }
    }
    scan_paths(&tables)
}

/// A table's material and how the position reads in it: mirrored where the
/// literal key is the mirrored one.
struct CanonicalRoot {
    ps: PieceConfig,
    mirrored: bool,
}

impl CanonicalRoot {
    fn view<'a>(&self, board: &'a Board) -> View<'a> {
        let v = View::new(board);
        if self.mirrored { v.mirror() } else { v }
    }
}

fn is_symmetric_material(ps: &PieceConfig) -> bool {
    let (mat_key, mir_key) = ps.material_keys();
    mat_key == mir_key
}

/// The canonical opposing pawn pair, if there is one.
fn opposing_pair(pos: View) -> Option<(Square, Square)> {
    let squares = |b: Bitboard| {
        let mut out = [Square::None; 16];
        let mut n = 0;
        for s in b {
            out[n] = s;
            n += 1;
        }
        (out, n)
    };
    let (ws, nw) = squares(pos.piece_bb(Piece::WhitePawn));
    let (bs, nb) = squares(pos.piece_bb(Piece::BlackPawn));
    PairGroup::canonical_pair(&ws[..nw], &bs[..nb])
}

/// The table's material for `pos`, less the castling rooks (`castling`) and
/// the pair, the colours flipped where the literal key is the mirrored one.
fn root_for(pos: View, castling: bool, pair: Option<(Square, Square)>) -> CanonicalRoot {
    let mut rooks = [Square::None; 4];
    let mut num_rooks = 0;
    let mut rights = [0usize; 2];
    if castling {
        for c in [Color::White, Color::Black] {
            for h_side in [false, true] {
                if pos.can_castle(c, h_side) {
                    rooks[num_rooks] = pos.castling_rook_square(c, h_side);
                    num_rooks += 1;
                    rights[c] += 1;
                }
            }
        }
    }
    let (pw, pb) = pair.unwrap_or((Square::None, Square::None));

    let mut pieces = [Piece::None; MAX_MAN];
    let mut n = 0;
    let mut key = MaterialKey::default();
    for pc in TABLE_PIECES {
        for s in pos.piece_bb(pc) {
            if s == pw || s == pb || rooks[..num_rooks].contains(&s) {
                continue;
            }
            pieces[n] = pc;
            n += 1;
            key.add_piece(pc);
        }
    }

    let mut ps = PieceConfig::new(&pieces[..n], rights);
    if pair.is_some() {
        ps.mark_opposing_pair();
        key.add_pair();
    }
    if num_rooks > 0 {
        key.add_castling(rights[0], rights[1]);
    }
    let mirrored = key != ps.base_material_key();
    CanonicalRoot { ps, mirrored }
}

/// The position as its tables index it: a castling or opposing-pair table
/// where one applies, else the material's own.
fn canonical_root(board: &Board) -> CanonicalRoot {
    let pos = View::new(board);
    let castling = pos.any_castling();
    if let Some(pair) = opposing_pair(pos) {
        let r = root_for(pos, castling, Some(pair));
        if open_wdl(&r.ps).is_some() {
            return r;
        }
    }
    root_for(pos, castling, None)
}

/// The frame to read and whether it is mirrored; a dropped frame of symmetric
/// material is read as its mirror. Not readable: no stored frame, or a
/// loss-only frame asked for a win.
struct FrameRead {
    frame: Color,
    mirror: bool,
    readable: bool,
}

fn locate_frame<P>(f: &TableFile<P>, ps: &PieceConfig, stm: Color, wdl: WdlEntry) -> FrameRead {
    let priced = |frame: Color| !(f.is_loss_only[frame] && matches!(wdl, WdlEntry::Win | WdlEntry::CursedWin));
    if !f.is_dropped[stm] {
        return FrameRead { frame: stm, mirror: false, readable: priced(stm) };
    }
    if !is_symmetric_material(ps) {
        return FrameRead { frame: stm, mirror: false, readable: false };
    }
    FrameRead { frame: !stm, mirror: true, readable: priced(!stm) }
}

/// The frame's view of the board.
fn frame_view<'a>(c: &CanonicalRoot, fr: &FrameRead, board: &'a Board) -> View<'a> {
    let v = c.view(board);
    if fr.mirror { v.mirror() } else { v }
}

pub const MAX_DERIVE_DEPTH: i32 = 64;

pub const fn wdl_rank(w: WdlEntry) -> i32 {
    match w {
        WdlEntry::Win => 4,
        WdlEntry::CursedWin => 3,
        WdlEntry::Draw => 2,
        WdlEntry::BlessedLoss => 1,
        WdlEntry::Lose => 0,
        WdlEntry::Illegal => -1,
    }
}

const fn stored_rank(s: WdlStored) -> i32 {
    match s {
        WdlStored::Lose => 0,
        WdlStored::BoundaryLoss => 1,
        WdlStored::BlessedLoss => 2,
        WdlStored::Draw => 3,
        WdlStored::CursedWin => 4,
        WdlStored::BoundaryWin => 5,
        WdlStored::Win => 6,
        WdlStored::Illegal => -1,
    }
}

pub const fn invert_wdl(w: WdlEntry) -> WdlEntry {
    match w {
        WdlEntry::Win => WdlEntry::Lose,
        WdlEntry::CursedWin => WdlEntry::BlessedLoss,
        WdlEntry::Draw => WdlEntry::Draw,
        WdlEntry::BlessedLoss => WdlEntry::CursedWin,
        WdlEntry::Lose => WdlEntry::Win,
        WdlEntry::Illegal => WdlEntry::Illegal,
    }
}

/// A stored code from the opponent's side, boundary codes included: a win or
/// loss at the 50-move edge is the cursed side of it one ply earlier.
const fn invert_stored(s: WdlStored) -> WdlEntry {
    match s {
        WdlStored::Win => WdlEntry::Lose,
        WdlStored::CursedWin => WdlEntry::BlessedLoss,
        WdlStored::Draw => WdlEntry::Draw,
        WdlStored::BlessedLoss => WdlEntry::CursedWin,
        WdlStored::Lose => WdlEntry::Win,
        WdlStored::BoundaryLoss => WdlEntry::CursedWin,
        WdlStored::BoundaryWin => WdlEntry::BlessedLoss,
        WdlStored::Illegal => WdlEntry::Illegal,
    }
}

pub const fn fold_dtm_wdl(w: WdlEntry) -> WdlEntry {
    match w {
        WdlEntry::CursedWin => WdlEntry::Win,
        WdlEntry::BlessedLoss => WdlEntry::Lose,
        _ => w,
    }
}

pub const fn fold_50mr_wdl(w: WdlEntry) -> WdlEntry {
    match w {
        WdlEntry::CursedWin | WdlEntry::BlessedLoss => WdlEntry::Draw,
        _ => w,
    }
}

fn prefer_new(new_wdl: WdlEntry, new_dtz: u16, old_wdl: WdlEntry, old_dtz: u16) -> bool {
    let (rn, ro) = (wdl_rank(new_wdl), wdl_rank(old_wdl));
    if rn != ro {
        return rn > ro;
    }
    match new_wdl {
        WdlEntry::Win | WdlEntry::CursedWin => new_dtz < old_dtz,
        WdlEntry::Lose | WdlEntry::BlessedLoss => new_dtz > old_dtz,
        _ => false,
    }
}

fn below_pinned_class(pinned: WdlEntry, offer: WdlEntry) -> bool {
    pinned != WdlEntry::Illegal && wdl_rank(offer) < wdl_rank(pinned)
}

/// A move to `child_wdl` at `dtz`, offered to the mover's best: past 100
/// plies a decisive class turns cursed.
fn offer_dtz(child_wdl: WdlEntry, dtz: u16, best_wdl: &mut WdlEntry, best_dtz: &mut u16) {
    let mut my_wdl = invert_wdl(child_wdl);
    if dtz > DTZ_MAX_NON_CURSED {
        my_wdl = match my_wdl {
            WdlEntry::Win => WdlEntry::CursedWin,
            WdlEntry::Lose => WdlEntry::BlessedLoss,
            w => w,
        };
    }
    if prefer_new(my_wdl, dtz, *best_wdl, *best_dtz) {
        *best_wdl = my_wdl;
        *best_dtz = dtz;
    }
}

const fn dtz_lift(my_wdl: WdlEntry) -> WdlEntry {
    if matches!(my_wdl, WdlEntry::Lose) { WdlEntry::BlessedLoss } else { my_wdl }
}

pub const fn is_win_class(w: WdlEntry) -> bool {
    matches!(w, WdlEntry::Win | WdlEntry::CursedWin)
}

const fn child_class_is_forced(pinned: WdlEntry) -> bool {
    matches!(pinned, WdlEntry::Lose)
}

fn dtm50_layer_pinned_by_dtm(wdl: WdlEntry, dtm: u16, rule50: u32) -> bool {
    is_decisive_class(wdl) && rule50 + dtm as u32 <= DTZ_MAX_NON_CURSED as u32
}

fn layer_busted_by_dtz(wdl: WdlEntry, dtz: u16, rule50: u32) -> bool {
    is_decisive_class(wdl) && rule50 + dtz as u32 > DTZ_MAX_NON_CURSED as u32
}

/// The children a minimax couldn't value: a class it skipped, or none known.
struct SkippedChildren {
    best_rank: i32,
    any_unknown: bool,
}

impl SkippedChildren {
    fn new() -> Self {
        Self { best_rank: -1, any_unknown: false }
    }

    fn of_class(&mut self, my_wdl: WdlEntry) {
        self.best_rank = self.best_rank.max(wdl_rank(my_wdl));
    }

    fn of_dtz_class(&mut self, my_wdl: WdlEntry) {
        self.of_class(dtz_lift(my_wdl));
    }

    fn unknown(&mut self) {
        self.any_unknown = true;
    }

    /// Whether a skipped child could have bettered `best_wdl`.
    fn unpin(&self, best_wdl: WdlEntry) -> bool {
        self.any_unknown || self.best_rank >= wdl_rank(best_wdl)
    }
}

pub fn is_bare_kings(board: &Board) -> bool {
    board.occupancies().popcount() == 2
}

pub fn is_zeroing(board: &Board, m: Move) -> bool {
    m.is_capture() || board.moved_piece(m).piece_type() == PieceType::Pawn
}

/// A capture or promotion: zeroing, but not a mere pawn push.
pub const fn is_conversion(m: Move) -> bool {
    m.is_promotion() || m.is_capture()
}

/// The legal moves. The generator's en passant captures are checked again.
pub fn legal_moves(board: &Board) -> MoveList {
    let mut moves = board.generate_all_moves();
    let mut i = 0;
    while i < moves.len() {
        let m = moves[i].mv;
        if m.is_en_passant() && !board.is_legal(m) {
            moves.remove(i);
        } else {
            i += 1;
        }
    }
    moves
}

pub fn is_checkmate(board: &Board) -> bool {
    board.in_check() && legal_moves(board).is_empty()
}

pub fn castling_count(board: &Board) -> u32 {
    board.castling().raw().count_ones()
}

/// The child is on the board; `parent_castling` is the parent's count of rights.
pub fn is_cursed_exit(parent_castling: u32, child: &Board, child_wdl: WdlEntry) -> bool {
    matches!(child_wdl, WdlEntry::CursedWin | WdlEntry::BlessedLoss) && castling_count(child) < parent_castling
}

pub const fn quiet_move_dtz(child_dtz: u16, cursed_exit: bool, child_rounded: bool) -> u16 {
    let v = child_dtz.wrapping_add(1);
    if !cursed_exit {
        v
    } else if child_rounded {
        v.wrapping_add(1)
    } else {
        v | 1
    }
}

/// Calls `f(board, m, is_zeroing)` with each legal move made on the board
/// (ChessTB's Child_Pos), but en passant captures, which the tables (no en
/// passant square) don't have; true if `f` stopped.
fn visit_legal_moves(board: &mut Board, mut f: impl FnMut(&mut Board, Move, bool) -> bool) -> bool {
    for entry in legal_moves(board).iter() {
        let m = entry.mv;
        if m.is_en_passant() {
            continue;
        }
        let zeroing = is_zeroing(board, m);
        board.make_move(m, &mut NullBoardObserver);
        let stop = f(board, m, zeroing);
        board.undo_move(m);
        if stop {
            return true;
        }
    }
    false
}

/// Where en passant captures are the only moves, the table entry (no en
/// passant state) isn't one of the options.
fn has_non_ep_move(board: &mut Board) -> bool {
    visit_legal_moves(board, |_, _, _| true)
}

/// Calls `f(board, m, has_non_ep_move)` with each en passant capture made on
/// the board; true if `f` stopped. The parent's has_non_ep_move is computed
/// once, only if there is a legal en passant capture, and only `with_non_ep`.
fn visit_legal_ep_captures(
    board: &mut Board, with_non_ep: bool, mut f: impl FnMut(&mut Board, Move, bool) -> bool,
) -> bool {
    if board.en_passant() == Square::None {
        return false;
    }
    let mut non_ep: Option<bool> = None;
    for entry in legal_moves(board).iter() {
        let m = entry.mv;
        if !m.is_en_passant() {
            continue;
        }
        if with_non_ep && non_ep.is_none() {
            non_ep = Some(has_non_ep_move(board));
        }
        board.make_move(m, &mut NullBoardObserver);
        let stop = f(board, m, non_ep.unwrap_or(false));
        board.undo_move(m);
        if stop {
            return true;
        }
    }
    false
}

pub fn has_ep_square(board: &Board) -> bool {
    board.en_passant() != Square::None
}

/// A probe's answers, each where it has one.
#[derive(Copy, Clone)]
pub struct ProbeResult {
    pub wdl: WdlEntry,
    pub has_dtz: bool,
    pub dtz: u16,
    pub dtz_rounded: bool,
    pub has_dtc: bool,
    pub dtc_wdl: WdlEntry,
    pub dtc_order: u16,
    pub dtc: u16,
    pub has_dtm: bool,
    pub dtm: u16,
    pub has_dtm50: bool,
    pub dtm50_wdl: WdlEntry,
    pub dtm50: u16,
}

impl Default for ProbeResult {
    fn default() -> Self {
        Self {
            wdl: WdlEntry::Illegal,
            has_dtz: false,
            dtz: 0,
            dtz_rounded: false,
            has_dtc: false,
            dtc_wdl: WdlEntry::Illegal,
            dtc_order: 0,
            dtc: 0,
            has_dtm: false,
            dtm: 0,
            has_dtm50: false,
            dtm50_wdl: WdlEntry::Illegal,
            dtm50: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct Dtm50Result {
    wdl: WdlEntry,
    dtm: u16,
    has_dtz: bool,
    dtz: u16,
}

impl Dtm50Result {
    const NONE: Self = Self { wdl: WdlEntry::Illegal, dtm: 0, has_dtz: false, dtz: 0 };

    const fn of(wdl: WdlEntry, dtm: u16) -> Self {
        Self { wdl, dtm, has_dtz: false, dtz: 0 }
    }
}

/// The best capture or promotion: what a relaxed frame's stored lower bound
/// leaves out.
fn relax_bound_wdl(board: &mut Board, depth: i32) -> WdlEntry {
    if depth >= MAX_DERIVE_DEPTH {
        return WdlEntry::Illegal;
    }
    let mut best = WdlEntry::Lose;
    for entry in legal_moves(board).iter() {
        let m = entry.mv;
        if !m.is_promotion() && !m.is_capture() {
            continue;
        }
        board.make_move(m, &mut NullBoardObserver);
        let cw = if is_bare_kings(board) { WdlEntry::Draw } else { probe_wdl_impl(board, depth + 1) };
        board.undo_move(m);
        if cw == WdlEntry::Illegal {
            best = WdlEntry::Illegal;
            continue;
        }
        let mine = invert_wdl(cw);
        if mine == WdlEntry::Win {
            return WdlEntry::Win;
        }
        if best != WdlEntry::Illegal && wdl_rank(mine) > wdl_rank(best) {
            best = mine;
        }
    }
    best
}

/// The stored code, boundary codes kept, raised by a relaxed frame's bound:
/// what a derive reads for a quiet child (the same material, so a stored frame).
fn read_wdl_stored(board: &mut Board, depth: i32) -> WdlStored {
    let c = canonical_root(board);
    let Some(w) = open_wdl(&c.ps) else { return WdlStored::Illegal };
    let pos = c.view(board);
    let stm = pos.turn();
    let s = w.read(stm, pos);
    if !w.is_relaxed[stm] || s == WdlStored::Illegal || s == WdlStored::Win {
        return s;
    }
    let bound = relax_bound_wdl(board, depth);
    if bound == WdlEntry::Illegal {
        return WdlStored::Illegal;
    }
    let b = WdlStored::from_entry(bound);
    if stored_rank(b) > stored_rank(s) { b } else { s }
}

fn derive_wdl(board: &mut Board, depth: i32) -> WdlEntry {
    if depth >= MAX_DERIVE_DEPTH {
        return WdlEntry::Illegal;
    }

    let mut any_legal = false;
    let mut best = WdlEntry::Lose;
    let mut skipped = SkippedChildren::new();

    if visit_legal_moves(board, |board, _, c_is_zeroing| {
        any_legal = true;
        let mw = if is_bare_kings(board) {
            WdlEntry::Draw
        } else if c_is_zeroing {
            let cw = probe_wdl_impl(board, depth + 1);
            if cw == WdlEntry::Illegal {
                skipped.unknown();
                return false;
            }
            invert_wdl(cw)
        } else {
            let cs = read_wdl_stored(board, depth + 1);
            if cs == WdlStored::Illegal {
                skipped.unknown();
                return false;
            }
            invert_stored(cs)
        };
        if wdl_rank(mw) > wdl_rank(best) {
            best = mw;
        }
        best == WdlEntry::Win
    }) {
        return best;
    }
    if !any_legal {
        return if board.in_check() { WdlEntry::Lose } else { WdlEntry::Draw };
    }
    if skipped.unpin(best) {
        return WdlEntry::Illegal;
    }
    best
}

fn raise_by_bound(w: &WdlFile, frame: Color, stored: WdlEntry, board: &mut Board, depth: i32) -> WdlEntry {
    if !w.is_relaxed[frame] || stored == WdlEntry::Illegal || stored == WdlEntry::Win {
        return stored;
    }
    let bound = relax_bound_wdl(board, depth);
    if bound == WdlEntry::Illegal {
        return WdlEntry::Illegal;
    }
    if wdl_rank(bound) > wdl_rank(stored) { bound } else { stored }
}

pub fn probe_wdl_internal(board: &mut Board, depth: i32) -> WdlEntry {
    let c = canonical_root(board);
    let Some(w) = open_wdl(&c.ps) else { return WdlEntry::Illegal };

    let pos = c.view(board);
    let stm = pos.turn();
    if w.is_dropped[stm] {
        if !is_symmetric_material(&c.ps) {
            return derive_wdl(board, depth);
        }
        let mp = pos.mirror();
        let stored = wdl_from_storage(w.read(mp.turn(), mp));
        return raise_by_bound(&w, mp.turn(), stored, board, depth);
    }
    let stored = wdl_from_storage(w.read(stm, pos));
    raise_by_bound(&w, stm, stored, board, depth)
}

/// With the position's en passant captures. Where they are the only moves,
/// the stored result (which has no en passant square) isn't one of the options.
pub fn probe_wdl_impl(board: &mut Board, depth: i32) -> WdlEntry {
    let mut best = probe_wdl_internal(board, depth);
    if best == WdlEntry::Illegal || best == WdlEntry::Win || !has_ep_square(board) {
        return best;
    }

    let mut any_ep = false;
    let mut unresolved_ep = false;
    if !visit_legal_ep_captures(board, true, |board, _, non_ep| {
        let cw = if is_bare_kings(board) { WdlEntry::Draw } else { probe_wdl_internal(board, depth + 1) };
        if cw == WdlEntry::Illegal {
            unresolved_ep = true;
            return false;
        }
        let mine = invert_wdl(cw);
        if (!non_ep && !any_ep) || wdl_rank(mine) > wdl_rank(best) {
            best = mine;
        }
        any_ep = true;
        best == WdlEntry::Win
    }) && unresolved_ep
    {
        return WdlEntry::Illegal;
    }
    best
}

/// A DTZ frame that isn't stored (or, `zeroing_only`, a relaxed winning
/// frame's check for a zeroing move that gives 1): one ply of search.
fn derive_dtz(board: &mut Board, wdl: WdlEntry, depth: i32, zeroing_only: bool) -> Option<DtzValue> {
    if depth >= MAX_DERIVE_DEPTH {
        return None;
    }

    let mut any_legal = false;
    let mut best_wdl = WdlEntry::Lose;
    let mut best_dtz = 0;
    let mut rounded = false;
    let mut skipped = SkippedChildren::new();
    let castling = castling_count(board);

    if visit_legal_moves(board, |board, _, c_is_zeroing| {
        any_legal = true;

        let cw;
        let my_dtz;
        if is_bare_kings(board) {
            cw = WdlEntry::Draw;
            my_dtz = 1;
        } else if c_is_zeroing {
            if child_class_is_forced(wdl) {
                cw = WdlEntry::Win;
            } else {
                cw = probe_wdl_impl(board, depth + 1);
                if cw == WdlEntry::Illegal {
                    skipped.unknown();
                    return false;
                }
            }
            my_dtz = 1;
        } else {
            if zeroing_only {
                return false;
            }
            if child_class_is_forced(wdl) {
                cw = WdlEntry::Win;
            } else {
                cw = probe_wdl_internal(board, depth + 1);
                if cw == WdlEntry::Illegal {
                    skipped.unknown();
                    return false;
                }
                if below_pinned_class(wdl, dtz_lift(invert_wdl(cw))) {
                    return false;
                }
            }
            let Some(child_dtz) = probe_dtz_internal(board, cw, depth + 1) else {
                skipped.of_dtz_class(invert_wdl(cw));
                return false;
            };
            my_dtz = quiet_move_dtz(child_dtz.dtz, is_cursed_exit(castling, board, cw), child_dtz.rounded);
            rounded |= child_dtz.rounded;
        }

        offer_dtz(cw, my_dtz, &mut best_wdl, &mut best_dtz);
        is_win_class(wdl) && best_wdl == wdl && best_dtz == 1
    }) {
        return Some(DtzValue { dtz: best_dtz, rounded: false });
    }
    if !any_legal {
        return Some(DtzValue::default());
    }
    if skipped.unpin(best_wdl) {
        return None;
    }
    if zeroing_only || best_wdl == WdlEntry::Draw {
        return Some(DtzValue::default());
    }
    Some(DtzValue { dtz: best_dtz, rounded })
}

/// The DTZ file; a frame that isn't stored is derived, a relaxed winning
/// frame checked for a zeroing move that gives 1.
fn probe_dtz_internal(board: &mut Board, wdl: WdlEntry, depth: i32) -> Option<DtzValue> {
    let c = canonical_root(board);
    let d = open_dtz(&c.ps)?;

    let fr = locate_frame(&d, &c.ps, c.view(board).turn(), wdl);
    if !fr.readable {
        return derive_dtz(board, wdl, depth, false);
    }
    if d.is_relaxed[fr.frame] && is_win_class(wdl) {
        let derived = derive_dtz(board, wdl, depth, true);
        if derived.is_none_or(|v| v.dtz == 1) {
            return derived;
        }
    }
    Some(read_dtz(&d, fr.frame, frame_view(&c, &fr, board), wdl))
}

/// A stored unclocked DTM50 cell's DTZ, from h, the first clock at which the
/// position turns drawn: 102 - h; h == 0 (never drawn) a zeroing move now, or
/// mate; h == 1 nothing. Cursed classes have none.
fn dtz_from_draw_flip(h: u16, wdl: WdlEntry, board: &Board) -> Option<u16> {
    match wdl {
        WdlEntry::Draw | WdlEntry::Illegal => Some(0),
        WdlEntry::Win | WdlEntry::Lose => match h {
            0 => Some(if wdl == WdlEntry::Lose && is_checkmate(board) { 0 } else { 1 }),
            1 => None,
            _ => Some(DTZ_MAX_NON_CURSED + 2 - h),
        },
        _ => None,
    }
}

struct DtzMinimax {
    best_wdl: WdlEntry,
    best_dtz: u16,
    skipped: SkippedChildren,
}

impl DtzMinimax {
    fn new() -> Self {
        Self {
            best_wdl: WdlEntry::Lose,
            best_dtz: 0,
            skipped: SkippedChildren::new(),
        }
    }

    fn zeroing_child(&mut self, child_wdl: WdlEntry) {
        offer_dtz(child_wdl, 1, &mut self.best_wdl, &mut self.best_dtz);
    }

    fn quiet_child(&mut self, child_wdl: WdlEntry, child: Dtm50Result, cursed_exit: bool) {
        if child.has_dtz {
            offer_dtz(child_wdl, quiet_move_dtz(child.dtz, cursed_exit, false), &mut self.best_wdl, &mut self.best_dtz);
        } else {
            self.skipped.of_dtz_class(invert_wdl(child_wdl));
        }
    }

    fn unwalked(&mut self) {
        self.skipped.unknown();
    }

    fn finish(&self, out: &mut Dtm50Result, any_legal: bool) {
        if !any_legal {
            out.has_dtz = true;
            return;
        }
        if self.skipped.unpin(self.best_wdl) {
            return;
        }
        out.has_dtz = true;
        out.dtz = if self.best_wdl == WdlEntry::Draw { 0 } else { self.best_dtz };
    }
}

fn derive_dtm(board: &mut Board, wdl: WdlEntry, depth: i32) -> Option<u16> {
    if depth >= MAX_DERIVE_DEPTH {
        return None;
    }
    let pinned = fold_dtm_wdl(wdl);

    let mut any_legal = false;
    let mut best_wdl = WdlEntry::Lose;
    let mut best_dtm = 0u16;
    let mut skipped = SkippedChildren::new();

    if visit_legal_moves(board, |board, _, _| {
        any_legal = true;
        let cw;
        let cd;
        if is_bare_kings(board) {
            cw = WdlEntry::Draw;
            cd = 0;
        } else if has_ep_square(board) {
            let cr = probe_dtm_impl(board, IGNORE_50MR, depth + 1);
            if cr.wdl == WdlEntry::Illegal {
                skipped.unknown();
                return false;
            }
            if !cr.has_dtm {
                skipped.of_class(invert_wdl(fold_dtm_wdl(cr.wdl)));
                return false;
            }
            cw = cr.wdl;
            cd = cr.dtm;
        } else {
            if child_class_is_forced(pinned) {
                cw = WdlEntry::Win;
            } else {
                cw = probe_wdl_internal(board, depth + 1);
                if cw == WdlEntry::Illegal {
                    skipped.unknown();
                    return false;
                }
                if below_pinned_class(pinned, invert_wdl(fold_dtm_wdl(cw))) {
                    return false;
                }
            }
            let Some(child_dtm) = probe_dtm_internal(board, cw, depth + 1) else {
                skipped.of_class(invert_wdl(fold_dtm_wdl(cw)));
                return false;
            };
            cd = child_dtm;
        }

        let my_wdl = invert_wdl(fold_dtm_wdl(cw));
        let my_dtm = 1 + cd;
        if prefer_new(my_wdl, my_dtm, best_wdl, best_dtm) {
            best_wdl = my_wdl;
            best_dtm = my_dtm;
        }
        is_win_class(pinned) && best_wdl == pinned && best_dtm == 1
    }) {
        return Some(best_dtm);
    }
    if !any_legal {
        return Some(0);
    }
    if skipped.unpin(best_wdl) {
        return None;
    }
    Some(if is_decisive_class(best_wdl) { best_dtm } else { 0 })
}

fn derive_dtm50(board: &mut Board, wdl: WdlEntry, rule50: u32, depth: i32) -> Dtm50Result {
    if depth >= MAX_DERIVE_DEPTH {
        return Dtm50Result::NONE;
    }
    let pinned = fold_50mr_wdl(wdl);

    let mut any_legal = false;
    let mut best_wdl = WdlEntry::Lose;
    let mut best_dtm = 0u16;
    let mut skipped = SkippedChildren::new();

    if visit_legal_moves(board, |board, _, c_is_zeroing| {
        any_legal = true;
        let child_rule50 = if c_is_zeroing { 0 } else { rule50 + 1 };

        let cd = if is_bare_kings(board) {
            Dtm50Result::of(WdlEntry::Draw, 0)
        } else if child_rule50 >= DTM50_HMC_COUNT {
            Dtm50Result::of(if is_checkmate(board) { WdlEntry::Lose } else { WdlEntry::Draw }, 0)
        } else if has_ep_square(board) {
            let cr = probe_dtm_impl(board, child_rule50, depth + 1);
            if cr.wdl == WdlEntry::Illegal {
                skipped.unknown();
                return false;
            }
            if !cr.has_dtm50 {
                skipped.of_class(invert_wdl(fold_50mr_wdl(cr.wdl)));
                return false;
            }
            Dtm50Result::of(cr.dtm50_wdl, cr.dtm50)
        } else {
            let cw = if child_class_is_forced(wdl) {
                WdlEntry::Win
            } else {
                let cw = probe_wdl_internal(board, depth + 1);
                if cw == WdlEntry::Illegal {
                    skipped.unknown();
                    return false;
                }
                if below_pinned_class(wdl, invert_wdl(cw)) {
                    return false;
                }
                cw
            };
            let cd = probe_dtm50_internal(board, cw, child_rule50, depth + 1, false);
            if cd.wdl == WdlEntry::Illegal {
                skipped.of_class(invert_wdl(fold_50mr_wdl(cw)));
                return false;
            }
            cd
        };

        let my_wdl = invert_wdl(fold_50mr_wdl(cd.wdl));
        let my_dtm = 1 + cd.dtm;
        if prefer_new(my_wdl, my_dtm, best_wdl, best_dtm) {
            best_wdl = my_wdl;
            best_dtm = my_dtm;
        }
        is_win_class(pinned) && best_wdl == pinned && best_dtm == 1
    }) {
        return Dtm50Result::of(best_wdl, best_dtm);
    }
    if !any_legal {
        return Dtm50Result::of(if board.in_check() { WdlEntry::Lose } else { WdlEntry::Draw }, 0);
    }
    if skipped.unpin(best_wdl) {
        return Dtm50Result::NONE;
    }
    if is_decisive_class(best_wdl) {
        return Dtm50Result::of(best_wdl, best_dtm);
    }
    Dtm50Result::of(WdlEntry::Draw, 0)
}

/// The DTZ minimax needs every child, so a caller after DTM alone (`with_dtz`
/// false) skips it and stops at a mate in one.
fn derive_dtm50_flat(board: &mut Board, wdl: WdlEntry, depth: i32, with_dtz: bool) -> Dtm50Result {
    if depth >= MAX_DERIVE_DEPTH {
        return Dtm50Result::NONE;
    }
    let pinned = fold_dtm_wdl(wdl);
    let castling = castling_count(board);

    let mut any_legal = false;
    let mut best_wdl = WdlEntry::Lose;
    let mut best_dtm = 0u16;
    let mut dtz = DtzMinimax::new();
    let mut skipped = SkippedChildren::new();

    let pinned_mate = visit_legal_moves(board, |board, _, c_is_zeroing| {
        any_legal = true;
        let cw;
        let cd;
        if is_bare_kings(board) {
            cw = WdlEntry::Draw;
            cd = 0;
            dtz.zeroing_child(WdlEntry::Draw);
        } else if has_ep_square(board) {
            let cr = probe_dtm_impl(board, IGNORE_50MR, depth + 1);
            if cr.wdl == WdlEntry::Illegal {
                skipped.unknown();
                dtz.unwalked();
                return false;
            }
            dtz.zeroing_child(cr.wdl);
            if !cr.has_dtm {
                skipped.of_class(invert_wdl(fold_dtm_wdl(cr.wdl)));
                return false;
            }
            cw = cr.wdl;
            cd = cr.dtm;
        } else {
            let raw_wdl = if child_class_is_forced(wdl) {
                WdlEntry::Win
            } else {
                let raw = probe_wdl_internal(board, depth + 1);
                if raw == WdlEntry::Illegal {
                    skipped.unknown();
                    dtz.unwalked();
                    return false;
                }
                if below_pinned_class(pinned, invert_wdl(fold_dtm_wdl(raw))) {
                    return false;
                }
                raw
            };
            let child = probe_dtm50_internal(board, raw_wdl, IGNORE_50MR, depth + 1, with_dtz);
            if c_is_zeroing {
                dtz.zeroing_child(raw_wdl);
            } else {
                dtz.quiet_child(raw_wdl, child, is_cursed_exit(castling, board, raw_wdl));
            }
            if child.wdl == WdlEntry::Illegal {
                skipped.of_class(invert_wdl(fold_dtm_wdl(raw_wdl)));
                return false;
            }
            cw = child.wdl;
            cd = child.dtm;
        }

        let my_wdl = invert_wdl(fold_dtm_wdl(cw));
        let my_dtm = 1 + cd;
        if prefer_new(my_wdl, my_dtm, best_wdl, best_dtm) {
            best_wdl = my_wdl;
            best_dtm = my_dtm;
        }
        !with_dtz && is_win_class(pinned) && best_wdl == pinned && best_dtm == 1
    });

    let mut out = Dtm50Result::NONE;
    if pinned_mate {
        out = Dtm50Result::of(best_wdl, best_dtm);
    } else if !any_legal {
        out.wdl = if board.in_check() { WdlEntry::Lose } else { WdlEntry::Draw };
    } else if !skipped.unpin(best_wdl) {
        let decisive = is_decisive_class(best_wdl);
        out.wdl = if decisive { best_wdl } else { WdlEntry::Draw };
        out.dtm = if decisive { best_dtm } else { 0 };
    }
    if with_dtz {
        dtz.finish(&mut out, any_legal);
    }
    out
}

fn probe_dtm_internal(board: &mut Board, wdl: WdlEntry, depth: i32) -> Option<u16> {
    let c = canonical_root(board);
    let d = open_dtm(&c.ps)?;

    let fr = locate_frame(&d, &c.ps, c.view(board).turn(), wdl);
    if !fr.readable {
        return derive_dtm(board, wdl, depth);
    }
    Some(read_dtm(&d, fr.frame, frame_view(&c, &fr, board), wdl))
}

/// The DTM50 file at `rule50`, or (IGNORE_50MR) its unclocked layer: DTM, and
/// the DTZ from the same cell; an unstored frame derives the DTZ only `with_dtz`.
fn probe_dtm50_internal(board: &mut Board, wdl: WdlEntry, rule50: u32, depth: i32, with_dtz: bool) -> Dtm50Result {
    let flat = rule50 == IGNORE_50MR;
    debug_assert!(flat || rule50 < DTM50_HMC_COUNT);
    let c = canonical_root(board);
    let Some(m) = open_dtm50(&c.ps) else { return Dtm50Result::NONE };

    let fr = locate_frame(&m, &c.ps, c.view(board).turn(), wdl);
    if !fr.readable {
        return if flat {
            derive_dtm50_flat(board, wdl, depth, with_dtz)
        } else {
            derive_dtm50(board, wdl, rule50, depth)
        };
    }
    let cell: Dtm50Cell = read_dtm50(&m, fr.frame, frame_view(&c, &fr, board), wdl, rule50);

    let mut r = if cell.value == DRAWN_AT_LAYER {
        Dtm50Result::of(WdlEntry::Draw, 0)
    } else {
        Dtm50Result::of(if flat { wdl } else { fold_50mr_wdl(wdl) }, cell.value)
    };
    if flat && let Some(dtz) = dtz_from_draw_flip(cell.draw_flip, wdl, board) {
        r.has_dtz = true;
        r.dtz = dtz;
    }
    r
}

fn prefer_new_dtc(
    new_wdl: WdlEntry, new_order: u16, new_value: u16, old_wdl: WdlEntry, old_order: u16, old_value: u16,
) -> bool {
    let (rn, ro) = (wdl_rank(new_wdl), wdl_rank(old_wdl));
    if rn != ro {
        return rn > ro;
    }
    match new_wdl {
        WdlEntry::Win | WdlEntry::CursedWin => (new_order, new_value) < (old_order, old_value),
        WdlEntry::Lose | WdlEntry::BlessedLoss => (new_order, new_value) > (old_order, old_value),
        _ => false,
    }
}

/// The best DTZ over the children, ignoring the clock.
struct UnboundedFold {
    winning: bool,
    have: bool,
    drawn: bool,
    any_rounded: bool,
    best: u16,
}

impl UnboundedFold {
    const fn new(mover_class: WdlEntry) -> Self {
        Self {
            winning: is_win_class(mover_class),
            have: false,
            drawn: false,
            any_rounded: false,
            best: 0,
        }
    }

    fn zeroing_child(&mut self) {
        self.offer(1);
    }

    fn quiet_child(&mut self, child_dtz: u16, cursed_exit: bool, child_rounded: bool) {
        if child_dtz == DRAWN_AT_LAYER {
            self.drawn = true;
        } else {
            self.offer(quiet_move_dtz(child_dtz, cursed_exit, child_rounded));
            self.any_rounded |= child_rounded;
        }
    }

    const fn value(&self) -> u16 {
        if self.have && !self.drawn { self.best } else { DRAWN_AT_LAYER }
    }

    fn offer(&mut self, v: u16) {
        if !self.have || (if self.winning { v < self.best } else { v > self.best }) {
            self.best = v;
        }
        self.have = true;
    }
}

fn read_dtc_curve_at(board: &Board, wdl: WdlEntry) -> Option<DtcCurve> {
    let c = canonical_root(board);
    let d = open_dtc(&c.ps)?;
    let fr = locate_frame(&d, &c.ps, c.view(board).turn(), wdl);
    if !fr.readable {
        return None;
    }
    Some(read_dtc_curve(&d, fr.frame, frame_view(&c, &fr, board), wdl))
}

/// The child is on the board.
fn child_dtc_cell(board: &mut Board, cw: WdlEntry, child_rule50: u32, depth: i32) -> Option<DtcCell> {
    if !has_ep_square(board) {
        return probe_dtc_internal(board, cw, child_rule50, depth);
    }

    let cr = probe_dtc_impl(board, child_rule50, depth);
    if !cr.has_dtc || !cr.has_dtz {
        return None;
    }
    let mut cell = DtcCell {
        dtz: cr.dtz,
        dtz_rounded: cr.dtz_rounded,
        ..Default::default()
    };
    if cr.dtc_wdl != WdlEntry::Draw {
        cell.order = cr.dtc_order;
        cell.value = cr.dtc;
    }
    Some(cell)
}

/// The child is on the board: does one of its en passant captures win for the
/// parent's side?
fn ep_conversion_wins(board: &mut Board, depth: i32) -> Option<bool> {
    let mut unknown = false;
    if visit_legal_ep_captures(board, false, |board, _, _| {
        let gw = if is_bare_kings(board) { WdlEntry::Draw } else { probe_wdl_impl(board, depth + 1) };
        if gw == WdlEntry::Illegal {
            unknown = true;
            return false;
        }
        fold_50mr_wdl(invert_wdl(gw)) == WdlEntry::Win
    }) {
        return Some(true);
    }
    if unknown { None } else { Some(false) }
}

fn derive_dtc_win(board: &mut Board, rule50: u32, depth: i32) -> Option<DtcCell> {
    let mut have = false;
    let mut best_order = 0u16;
    let mut best_value = 0u16;
    let budget_plies = dtc_budget_plies(rule50);
    let mut unbounded = UnboundedFold::new(WdlEntry::Win);
    let mut skipped = SkippedChildren::new();

    let pinned = visit_legal_moves(board, |board, m, c_is_zeroing| {
        let conversion = is_conversion(m);
        let push = !conversion && c_is_zeroing;
        if is_bare_kings(board) {
            return false;
        }

        let cw = probe_wdl_impl(board, depth + 1);
        if cw == WdlEntry::Illegal {
            skipped.unknown();
            return false;
        }
        if cw != WdlEntry::Lose {
            return false;
        }

        let mut order = 0u16;
        let mut value = 1u16;
        if conversion {
            unbounded.zeroing_child();
        } else {
            let child_rule50 = if push {
                0
            } else if rule50 == IGNORE_50MR {
                1
            } else {
                rule50 + 1
            };
            let Some(cell) = child_dtc_cell(board, cw, child_rule50, depth + 1) else {
                skipped.unknown();
                return false;
            };
            if push {
                unbounded.zeroing_child();
            } else {
                unbounded.quiet_child(cell.dtz, false, false);
            }
            if !cell.priced() {
                return false;
            }
            order = cell.order + push as u16;
            value = if push { 1 } else { cell.value + 1 };
        }
        if value as u32 > budget_plies {
            return false;
        }

        if !have || order < best_order || (order == best_order && value < best_value) {
            best_order = order;
            best_value = value;
            have = true;
        }
        best_order == 0 && best_value == 1
    });
    if !pinned && skipped.unpin(WdlEntry::Win) {
        return None;
    }

    let mut out = DtcCell {
        dtz: if pinned { 1 } else { unbounded.value() },
        ..Default::default()
    };
    if have {
        out.order = best_order;
        out.value = best_value;
    }
    Some(out)
}

fn derive_dtc_loss(board: &mut Board, rule50: u32, depth: i32) -> Option<DtcCell> {
    let mut worst = [0u16; DTC_PACK_LAYERS];
    let mut drawn = [false; DTC_PACK_LAYERS];
    let mut unbounded = UnboundedFold::new(WdlEntry::Lose);

    let raise = |worst: &mut [u16; DTC_PACK_LAYERS], drawn: &mut [bool; DTC_PACK_LAYERS], k: usize, val: u16| {
        if val > DTZ_MAX_NON_CURSED {
            drawn[k] = true;
        } else if val > worst[k] {
            worst[k] = val;
        }
    };

    let mut any_legal = false;

    if visit_legal_moves(board, |board, m, c_is_zeroing| {
        any_legal = true;

        // Only a capture can bare the kings, so `conversion` already covers that.
        if is_conversion(m) {
            for k in 0..DTC_PACK_LAYERS {
                raise(&mut worst, &mut drawn, k, 1);
            }
            unbounded.zeroing_child();
            return false;
        }

        let push = c_is_zeroing;

        if has_ep_square(board) {
            let Some(ep_wins) = ep_conversion_wins(board, depth + 1) else { return true };
            if ep_wins {
                for k in 0..DTC_PACK_LAYERS {
                    raise(&mut worst, &mut drawn, k, 1);
                }
                unbounded.zeroing_child();
                return false;
            }
        }

        let Some(curve) = read_dtc_curve_at(board, WdlEntry::Win) else { return true };

        if push {
            unbounded.zeroing_child();
        } else {
            unbounded.quiet_child(curve.value[DTC_BUDGET_LAYERS], false, false);
        }
        for k in 0..DTC_PACK_LAYERS {
            let v = curve.value[k];
            if v == DRAWN_AT_LAYER {
                drawn[k] = true;
            } else {
                raise(&mut worst, &mut drawn, k, if push { 1 } else { v + 1 });
            }
        }
        false
    }) {
        return None;
    }

    if !any_legal {
        return Some(DtcCell { order: 0, value: 0, dtz: 0, dtz_rounded: false });
    }

    let mut out = DtcCell { dtz: unbounded.value(), ..Default::default() };
    let budget_plies = dtc_budget_plies(rule50);
    if let Some(k) = (0..DTC_PACK_LAYERS).find(|&k| !drawn[k] && worst[k] as u32 <= budget_plies) {
        out.order = k as u16;
        out.value = worst[k];
    }
    Some(out)
}

fn derive_dtc_cursed(board: &mut Board, wdl: WdlEntry, depth: i32) -> Option<DtcCell> {
    let winning = wdl == WdlEntry::CursedWin;
    let castling = castling_count(board);
    let mut unbounded = UnboundedFold::new(wdl);
    let mut any_legal = false;
    let mut skipped = SkippedChildren::new();

    let pinned = visit_legal_moves(board, |board, m, c_is_zeroing| {
        any_legal = true;

        let conversion = is_conversion(m);
        let push = !conversion && c_is_zeroing;
        if is_bare_kings(board) {
            // Baring the kings takes a capture, and from a blessed loss such a
            // capture would be an outright draw, not a loss.
            return false;
        }

        let cw = probe_wdl_impl(board, depth + 1);
        if cw == WdlEntry::Illegal {
            skipped.unknown();
            return false;
        }
        if winning && cw != WdlEntry::Lose && cw != WdlEntry::BlessedLoss {
            return false;
        }
        // A blessed loss counts only the moves that keep it blessed: into a
        // cursed win, or quietly into a win already at the horizon.
        if !winning && cw != WdlEntry::CursedWin && (conversion || push) {
            return false;
        }

        if conversion || push {
            unbounded.zeroing_child();
            return winning && invert_wdl(cw) == wdl;
        }
        let Some(cell) = probe_dtc_internal(board, cw, IGNORE_50MR, depth + 1) else {
            skipped.unknown();
            return false;
        };
        if !winning && cw == WdlEntry::Win && cell.dtz < DTZ_MAX_NON_CURSED {
            return false;
        }
        unbounded.quiet_child(cell.dtz, is_cursed_exit(castling, board, cw), cell.dtz_rounded);
        false
    });
    if !pinned && skipped.unpin(wdl) {
        return None;
    }

    let mut out = DtcCell::default();
    if any_legal {
        out.dtz = if pinned { 1 } else { unbounded.value() };
    }
    out.dtz_rounded = !pinned && unbounded.any_rounded;
    Some(out)
}

fn derive_dtc(board: &mut Board, wdl: WdlEntry, rule50: u32, depth: i32) -> Option<DtcCell> {
    if depth >= MAX_DERIVE_DEPTH {
        return None;
    }
    match wdl {
        WdlEntry::Win => derive_dtc_win(board, rule50, depth),
        WdlEntry::Lose => derive_dtc_loss(board, rule50, depth),
        _ => derive_dtc_cursed(board, wdl, depth),
    }
}

fn probe_dtc_internal(board: &mut Board, wdl: WdlEntry, rule50: u32, depth: i32) -> Option<DtcCell> {
    let c = canonical_root(board);
    let d = open_dtc(&c.ps)?;

    let fr = locate_frame(&d, &c.ps, c.view(board).turn(), wdl);
    if !fr.readable {
        return derive_dtc(board, wdl, rule50, depth);
    }
    Some(read_dtc(&d, fr.frame, frame_view(&c, &fr, board), wdl, rule50))
}

fn fill_dtm50_from_bounds(r: &mut ProbeResult, rule50: u32) {
    debug_assert!(rule50 < DTM50_HMC_COUNT);
    if fold_50mr_wdl(r.wdl) == WdlEntry::Draw || (r.has_dtz && layer_busted_by_dtz(r.wdl, r.dtz, rule50)) {
        r.dtm50_wdl = WdlEntry::Draw;
    } else if r.has_dtm && dtm50_layer_pinned_by_dtm(r.wdl, r.dtm, rule50) {
        r.dtm50_wdl = r.wdl;
        r.dtm50 = r.dtm;
    } else {
        return;
    }
    r.has_dtm50 = true;
}

fn fill_dtc_from_bounds(r: &mut ProbeResult, has_pawns: bool, rule50: u32) {
    if rule50 == IGNORE_50MR {
        return;
    }
    if !is_decisive_class(r.wdl) {
        r.has_dtc = true;
        r.dtc_wdl = WdlEntry::Draw;
        return;
    }
    if !r.has_dtz {
        return;
    }

    let busted = r.dtz as u32 > dtc_budget_plies(rule50);
    if has_pawns && !busted {
        return;
    }
    r.has_dtc = true;
    r.dtc_wdl = if busted { WdlEntry::Draw } else { r.wdl };
    r.dtc = if busted { 0 } else { r.dtz };
}

/// DTM, and DTM50 at `rule50` (none at IGNORE_50MR). The unclocked layer first
/// (DTM, and the DTZ with it): most clocks are settled by its bounds (the mate
/// fits in what the clock leaves, or the conversion doesn't); only the rest
/// read the clocked layer. Without DTM50 only the bounds can answer, from DTM
/// and DTZ. En passant captures included.
pub fn probe_dtm_impl(board: &mut Board, rule50: u32, depth: i32) -> ProbeResult {
    let mut r = ProbeResult::default();
    let c = canonical_root(board);
    let clocked = rule50 != IGNORE_50MR;
    let rule50_drawn = clocked && rule50 >= DTM50_HMC_COUNT;
    let live = clocked && !rule50_drawn;
    r.wdl = probe_wdl_internal(board, depth);
    if r.wdl == WdlEntry::Illegal {
        return r;
    }

    if r.wdl == WdlEntry::Draw {
        r.has_dtm = true;
    } else if open_dtm50(&c.ps).is_some() {
        let d50 = probe_dtm50_internal(board, r.wdl, IGNORE_50MR, depth, live);
        r.dtm = d50.dtm;
        r.has_dtm = d50.wdl != WdlEntry::Illegal;
        r.dtz = d50.dtz;
        r.has_dtz = d50.has_dtz;
        if live {
            fill_dtm50_from_bounds(&mut r, rule50);
        }
        if live && !r.has_dtm50 {
            let d50r = probe_dtm50_internal(board, r.wdl, rule50, depth, false);
            r.dtm50_wdl = d50r.wdl;
            r.dtm50 = d50r.dtm;
            r.has_dtm50 = d50r.wdl != WdlEntry::Illegal;
        }
    } else {
        if let Some(dtm) = probe_dtm_internal(board, r.wdl, depth) {
            r.dtm = dtm;
            r.has_dtm = true;
        }
        if live {
            fill_dtm50_from_bounds(&mut r, rule50);
        }
        if live && !r.has_dtm50 {
            if board.pieces(PieceType::Pawn).0 != 0
                && let Some(cell) = probe_dtc_internal(board, r.wdl, IGNORE_50MR, depth)
            {
                r.dtz = cell.dtz;
                r.has_dtz = true;
            }
            if !r.has_dtz
                && let Some(dtz) = probe_dtz_internal(board, r.wdl, depth)
            {
                r.dtz = dtz.dtz;
                r.has_dtz = true;
            }
            fill_dtm50_from_bounds(&mut r, rule50);
        }
    }
    if clocked && (r.wdl == WdlEntry::Draw || rule50_drawn) {
        // Past the clock only mate is decided.
        r.dtm50_wdl = if r.wdl == WdlEntry::Lose && is_checkmate(board) { WdlEntry::Lose } else { WdlEntry::Draw };
        r.has_dtm50 = true;
    }
    if !has_ep_square(board) {
        return r;
    }

    let mut best = r;
    let mut best_dtm_wdl = fold_dtm_wdl(r.wdl);
    let mut best_dtm = if r.has_dtm { r.dtm } else { 0 };
    let mut best_dtm50_wdl = if r.has_dtm50 { r.dtm50_wdl } else { fold_50mr_wdl(r.wdl) };
    let mut best_dtm50 = if r.has_dtm50 { r.dtm50 } else { 0 };
    let mut any_ep = false;
    if rule50_drawn {
        // Even an EP conversion costs a ply; the claim is already available.
        best.dtm50_wdl = WdlEntry::Draw;
        best.has_dtm50 = true;
    }
    if visit_legal_ep_captures(board, true, |board, _, non_ep| {
        let cr = if is_bare_kings(board) {
            ProbeResult {
                wdl: WdlEntry::Draw,
                has_dtm: best.has_dtm,
                dtm50_wdl: WdlEntry::Draw,
                has_dtm50: best.has_dtm50,
                ..Default::default()
            }
        } else {
            probe_dtm_impl(board, if live { 0 } else { IGNORE_50MR }, depth + 1)
        };
        if cr.wdl == WdlEntry::Illegal {
            return true;
        }
        let my_wdl = invert_wdl(cr.wdl);
        if !non_ep && !any_ep {
            best.wdl = my_wdl;
            best_dtm_wdl = fold_dtm_wdl(my_wdl);
            best_dtm = if cr.has_dtm { 1 + cr.dtm } else { 0 };
            best.dtm = if cr.has_dtm && is_decisive_class(best_dtm_wdl) { best_dtm } else { 0 };
            best.has_dtm = cr.has_dtm;
            if live {
                best_dtm50_wdl = if cr.has_dtm50 { invert_wdl(cr.dtm50_wdl) } else { WdlEntry::Illegal };
                best_dtm50 = if cr.has_dtm50 { 1 + cr.dtm50 } else { 0 };
                best.dtm50_wdl = best_dtm50_wdl;
                best.dtm50 = if cr.has_dtm50 && is_decisive_class(best_dtm50_wdl) { best_dtm50 } else { 0 };
                best.has_dtm50 = cr.has_dtm50;
            }
            any_ep = true;
            return false;
        }
        any_ep = true;
        if wdl_rank(my_wdl) > wdl_rank(best.wdl) {
            best.wdl = my_wdl;
        }
        if !cr.has_dtm {
            best.has_dtm = false;
        } else if best.has_dtm {
            let my_dtm_wdl = fold_dtm_wdl(my_wdl);
            let my_dtm = 1 + cr.dtm;
            if prefer_new(my_dtm_wdl, my_dtm, best_dtm_wdl, best_dtm) {
                best_dtm_wdl = my_dtm_wdl;
                best_dtm = my_dtm;
                best.dtm = if is_decisive_class(my_dtm_wdl) { my_dtm } else { 0 };
            }
        }
        if !live {
            return false;
        }
        if !cr.has_dtm50 {
            best.has_dtm50 = false;
        } else if best.has_dtm50 {
            let my_dtm50_wdl = invert_wdl(cr.dtm50_wdl);
            let my_dtm50 = 1 + cr.dtm50;
            if prefer_new(my_dtm50_wdl, my_dtm50, best_dtm50_wdl, best_dtm50) {
                best_dtm50_wdl = my_dtm50_wdl;
                best_dtm50 = my_dtm50;
                best.dtm50_wdl = my_dtm50_wdl;
                best.dtm50 = if is_decisive_class(my_dtm50_wdl) { my_dtm50 } else { 0 };
            }
        }
        false
    }) {
        return ProbeResult::default();
    }
    if any_ep { best } else { r }
}

/// DTZ, and DTC at `rule50` (none at IGNORE_50MR). A pawn material's DTC cell
/// carries both; otherwise the DTC comes from DTZ's bounds. En passant
/// captures included.
pub fn probe_dtc_impl(board: &mut Board, rule50: u32, depth: i32) -> ProbeResult {
    let mut r = ProbeResult::default();
    let has_pawns = board.pieces(PieceType::Pawn).0 != 0;
    let clocked = rule50 != IGNORE_50MR;
    let rule50_drawn = clocked && rule50 >= DTM50_HMC_COUNT;
    let live = clocked && !rule50_drawn;
    r.wdl = probe_wdl_internal(board, depth);
    if r.wdl == WdlEntry::Illegal {
        return r;
    }

    // A DTC cell serves its DTZ row too. Clocked, it is read first for its
    // DTC; unclocked, after DTM50's exact DTZ.
    let from_cell = |board: &mut Board, r: &mut ProbeResult, at: u32| {
        if let Some(cell) = probe_dtc_internal(board, r.wdl, at, depth) {
            if clocked {
                r.dtc_wdl = if cell.priced() { r.wdl } else { WdlEntry::Draw };
                r.dtc_order = if cell.priced() { cell.order } else { 0 };
                r.dtc = if cell.priced() { cell.value } else { 0 };
                r.has_dtc = true;
            }
            r.dtz = cell.dtz;
            r.dtz_rounded = cell.dtz_rounded;
            r.has_dtz = true;
        }
    };
    if r.wdl == WdlEntry::Draw {
        r.has_dtz = true;
        if clocked {
            r.dtc_wdl = WdlEntry::Draw;
            r.has_dtc = true;
        }
    } else {
        if has_pawns && clocked {
            from_cell(board, &mut r, rule50);
        }
        if !r.has_dtz {
            let d50 = probe_dtm50_internal(board, r.wdl, IGNORE_50MR, depth, true);
            r.dtz = d50.dtz;
            r.has_dtz = d50.has_dtz;
        }
        if !r.has_dtz && has_pawns && !clocked {
            from_cell(board, &mut r, IGNORE_50MR);
        }
        if !r.has_dtz
            && let Some(dtz) = probe_dtz_internal(board, r.wdl, depth)
        {
            r.dtz = dtz.dtz;
            r.dtz_rounded = dtz.rounded;
            r.has_dtz = true;
        }
        if !r.has_dtc {
            fill_dtc_from_bounds(&mut r, has_pawns, rule50);
        }
    }
    if !has_ep_square(board) {
        return r;
    }

    let mut best = r;
    let mut best_dtz_wdl = r.wdl;
    let mut best_dtz = if r.has_dtz { r.dtz } else { 0 };
    let mut best_dtc_wdl = if r.has_dtc { r.dtc_wdl } else { r.wdl };
    let mut best_dtc_order = if r.has_dtc { r.dtc_order } else { 0 };
    let mut best_dtc = if r.has_dtc { r.dtc } else { 0 };
    let mut any_ep = false;
    if rule50_drawn {
        // Even an EP conversion costs a ply; the claim is already available.
        best.dtc_wdl = WdlEntry::Draw;
        best.has_dtc = true;
    }
    if visit_legal_ep_captures(board, true, |board, _, non_ep| {
        let cw = if is_bare_kings(board) { WdlEntry::Draw } else { probe_wdl_internal(board, depth + 1) };
        if cw == WdlEntry::Illegal {
            return true;
        }
        let my_wdl = invert_wdl(cw);
        if !non_ep && !any_ep {
            best.wdl = my_wdl;
            best_dtz_wdl = my_wdl;
            best_dtz = 1;
            best.dtz = if my_wdl == WdlEntry::Draw { 0 } else { 1 };
            best.dtz_rounded = false;
            best.has_dtz = true;
            if live {
                best_dtc_wdl = fold_50mr_wdl(my_wdl);
                best_dtc_order = 0;
                best_dtc = 1;
                best.dtc_wdl = best_dtc_wdl;
                best.dtc_order = 0;
                best.dtc = if best_dtc_wdl == WdlEntry::Draw { 0 } else { 1 };
                best.has_dtc = true;
            }
            any_ep = true;
            return false;
        }
        any_ep = true;
        if wdl_rank(my_wdl) > wdl_rank(best.wdl) {
            best.wdl = my_wdl;
        }
        if wdl_rank(my_wdl) > wdl_rank(best_dtz_wdl) || (best.has_dtz && prefer_new(my_wdl, 1, best_dtz_wdl, best_dtz))
        {
            best_dtz_wdl = my_wdl;
            best_dtz = 1;
            best.dtz = if my_wdl == WdlEntry::Draw { 0 } else { 1 };
            best.dtz_rounded = false;
            best.has_dtz = true;
        }
        if !live {
            return false;
        }
        let my_dtc_wdl = fold_50mr_wdl(my_wdl);
        if wdl_rank(my_dtc_wdl) > wdl_rank(best_dtc_wdl)
            || (best.has_dtc && prefer_new_dtc(my_dtc_wdl, 0, 1, best_dtc_wdl, best_dtc_order, best_dtc))
        {
            best_dtc_wdl = my_dtc_wdl;
            best_dtc_order = 0;
            best_dtc = 1;
            best.dtc_wdl = my_dtc_wdl;
            best.dtc_order = 0;
            best.dtc = if my_dtc_wdl != WdlEntry::Draw { 1 } else { 0 };
            best.has_dtc = true;
        }
        false
    }) {
        return ProbeResult::default();
    }
    if any_ep { best } else { r }
}
