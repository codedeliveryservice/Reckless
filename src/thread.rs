use std::{
    ops::{Index, IndexMut},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
    },
};

#[cfg(target_arch = "wasm32")]
thread_local! {
    pub static WASM_CALLBACK: std::cell::RefCell<Option<js_sys::Function>> =
        const { std::cell::RefCell::new(None) };
    pub static WASM_DISPATCH: std::cell::RefCell<Option<js_sys::Function>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub static WORKERS_REMAINING: AtomicUsize = AtomicUsize::new(0);

use crate::{
    board::Board,
    history::{ContinuationCorrectionHistory, ContinuationHistory, CorrectionHistory, NoisyHistory, QuietHistory},
    nnue::{Network, ParametersHandle},
    numa::{NumaConfig, NumaReplicable, NumaReplicated, NumaReplicatedAccessToken, NumaReplicationContext},
    stack::Stack,
    threadpool::ThreadPool,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::{MAX_MOVES, MAX_PLY, Move, Score, normalize_to_cp},
};

pub trait UciWriter: Send {
    fn write_line(&mut self, line: &str);
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn take(&mut self) -> String {
        String::new()
    }
}

pub struct StdoutWriter;

impl UciWriter for StdoutWriter {
    fn write_line(&mut self, line: &str) {
        println!("{line}");
    }
}

#[derive(Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct BufferWriter {
    pub buffer: String,
}

impl UciWriter for BufferWriter {
    fn write_line(&mut self, line: &str) {
        self.buffer.push_str(line);
        self.buffer.push('\n');
        #[cfg(target_arch = "wasm32")]
        WASM_CALLBACK.with(|cb| {
            if let Some(f) = &*cb.borrow() {
                let _ = f.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(line));
            }
        });
    }

    fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }
}

#[repr(align(64))]
struct AlignedAtomicU64 {
    inner: AtomicU64,
}

pub struct Counter {
    shards: Box<[AlignedAtomicU64]>,
}

unsafe impl Sync for Counter {}

impl Counter {
    pub fn aggregate(&self) -> u64 {
        self.shards.iter().map(|shard| shard.inner.load(Ordering::Relaxed)).sum()
    }

    pub fn get(&self, id: usize) -> u64 {
        self.shards[id].inner.load(Ordering::Relaxed)
    }

    pub fn increment(&self, id: usize) {
        self.shards[id].inner.store(self.shards[id].inner.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        for shard in &self.shards {
            shard.inner.store(0, Ordering::Relaxed);
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            shards: std::iter::from_fn(|| Some(AlignedAtomicU64 { inner: AtomicU64::new(0) }))
                .take(ThreadPool::available_threads())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

pub struct Status {
    inner: AtomicUsize,
}

impl Status {
    pub const STOPPED: usize = 0;
    pub const RUNNING: usize = 1;

    pub fn get(&self) -> usize {
        self.inner.load(Ordering::Acquire)
    }

    pub fn set(&self, status: usize) {
        self.inner.store(status, Ordering::Release);
    }
}

impl Clone for Status {
    fn clone(&self) -> Self {
        Self { inner: AtomicUsize::new(self.inner.load(Ordering::Relaxed)) }
    }
}

impl Default for Status {
    fn default() -> Self {
        Self { inner: AtomicUsize::new(Self::STOPPED) }
    }
}

#[derive(Default)]
pub struct SharedCorrectionHistory {
    pub pawn: CorrectionHistory,
    pub non_pawn: [CorrectionHistory; 2],
}

impl NumaReplicable for SharedCorrectionHistory {
    fn allocate() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

pub struct SharedContext {
    pub tt: TranspositionTable,
    pub status: Status,
    pub nodes: Counter,
    pub tb_hits: Counter,
    pub stop_probing_tb: AtomicBool,
    pub root_in_tb: AtomicBool,
    pub soft_stop_votes: AtomicUsize,
    pub best_stats: [AtomicU32; MAX_MOVES],
    pub history: Arc<NumaReplicated<SharedCorrectionHistory>>,
    pub parameters: Arc<NumaReplicated<ParametersHandle>>,
    pub numa_context: Arc<NumaReplicationContext>,
}

impl Default for SharedContext {
    fn default() -> Self {
        let numa_context = Arc::new(NumaReplicationContext::new(NumaConfig::from_system()));

        Self {
            tt: TranspositionTable::default(),
            status: Status::default(),
            nodes: Counter::default(),
            tb_hits: Counter::default(),
            stop_probing_tb: AtomicBool::new(false),
            root_in_tb: AtomicBool::new(false),
            soft_stop_votes: AtomicUsize::new(0),
            best_stats: [const { AtomicU32::new(0) }; MAX_MOVES],
            history: NumaReplicated::new(numa_context.clone()),
            parameters: NumaReplicated::new(numa_context.clone()),
            numa_context,
        }
    }
}

pub struct PlyArray<T, const N: usize> {
    data: [T; N],
}

impl<T, const N: usize> Index<isize> for PlyArray<T, N> {
    type Output = T;
    fn index(&self, index: isize) -> &T {
        &self.data[(index + 8) as usize]
    }
}

impl<T, const N: usize> IndexMut<isize> for PlyArray<T, N> {
    fn index_mut(&mut self, index: isize) -> &mut T {
        &mut self.data[(index + 8) as usize]
    }
}

impl<T: Copy + Default, const N: usize> Default for PlyArray<T, N> {
    fn default() -> Self {
        Self { data: [T::default(); N] }
    }
}

pub struct ThreadData {
    pub id: usize,
    pub shared: Arc<SharedContext>,
    pub corrhist: Arc<SharedCorrectionHistory>,
    pub board: Board,
    pub time_manager: TimeManager,
    pub stack: Box<Stack>,
    pub nnue: Network,
    pub root_moves: Vec<RootMove>,
    pub pv_table: PrincipalVariationTable,
    pub noisy_history: NoisyHistory,
    pub quiet_history: QuietHistory,
    pub continuation_history: ContinuationHistory,
    pub continuation_corrhist: ContinuationCorrectionHistory,
    pub best_move_changes: usize,
    pub optimism: [i32; 2],
    pub root_depth: i32,
    pub root_delta: i32,
    pub sel_depth: i32,
    pub completed_depth: i32,
    pub nmp_min_ply: i32,
    pub previous_best_score: i32,
    pub multi_pv: usize,
    pub pv_index: usize,
    pub pv_start: usize,
    pub pv_end: usize,
    pub cutoff_count: PlyArray<i32, { MAX_PLY + 16 }>,
    pub excluded: PlyArray<Move, { MAX_PLY + 16 }>,
    pub writer: Box<dyn UciWriter>,
}

impl ThreadData {
    pub fn new(shared: Arc<SharedContext>, numa_token: NumaReplicatedAccessToken) -> Self {
        let corrhist = shared.history.get(numa_token);
        let parameters = shared.parameters.get(numa_token);

        Self {
            id: 0,
            shared,
            corrhist,
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite, 0, 0),
            stack: Stack::new(),
            nnue: Network::new(parameters),
            root_moves: Vec::new(),
            pv_table: PrincipalVariationTable::default(),
            noisy_history: NoisyHistory::default(),
            quiet_history: QuietHistory::default(),
            continuation_history: ContinuationHistory::default(),
            continuation_corrhist: ContinuationCorrectionHistory::default(),
            best_move_changes: 0,
            optimism: [0; 2],
            root_depth: 0,
            root_delta: 0,
            sel_depth: 0,
            completed_depth: 0,
            nmp_min_ply: 0,
            previous_best_score: 0,
            multi_pv: 1,
            pv_index: 0,
            pv_start: 0,
            pv_end: 0,
            cutoff_count: PlyArray::default(),
            excluded: PlyArray::default(),
            #[cfg(not(target_arch = "wasm32"))]
            writer: Box::new(StdoutWriter),
            #[cfg(target_arch = "wasm32")]
            writer: Box::new(BufferWriter::default()),
        }
    }

    pub fn nodes(&self) -> u64 {
        self.shared.nodes.get(self.id)
    }

    pub fn corrhist(&self) -> &SharedCorrectionHistory {
        &self.corrhist
    }

    pub fn conthist(&self, ply: isize, index: isize, mv: Move) -> i32 {
        self.continuation_history.get(self.stack[ply - index].conthist, self.board.piece_on(mv.from()), mv.to())
    }

    pub fn print_uci_info(&mut self, depth: i32) {
        if self.root_moves.is_empty() {
            self.print_uci_no_move();
            return;
        }

        let elapsed = self.time_manager.elapsed();
        let nps = self.shared.nodes.aggregate() as f64 / elapsed.as_secs_f64();
        let ms = elapsed.as_millis();

        for pv_index in 0..self.multi_pv {
            let root_move = &self.root_moves[pv_index];

            let updated = root_move.score != -Score::INFINITE;

            if depth == 1 && !updated && pv_index > 0 {
                continue;
            }

            let depth = if updated { depth } else { (depth - 1).max(1) };
            let mut score = if updated { root_move.display_score } else { root_move.previous_score };

            let mut upperbound = root_move.upperbound;
            let mut lowerbound = root_move.lowerbound;

            if self.shared.root_in_tb.load(Ordering::Relaxed) {
                // Cursed win/loss due to 50-move rule
                let is_cursed = root_move.tb_rank > 0 && root_move.tb_rank < 900
                    || root_move.tb_rank < 0 && root_move.tb_rank > -900;

                if is_cursed {
                    upperbound = false;
                    lowerbound = false;
                    score = 0;
                } else if score.abs() <= Score::TB_WIN {
                    upperbound = false;
                    lowerbound = false;
                    score = root_move.tb_score;
                }
            }

            let mut formatted_score = match score.abs() {
                s if s < Score::TB_WIN_IN_MAX => {
                    format!("cp {}", normalize_to_cp(score, &self.board))
                }
                s if s <= Score::TB_WIN => {
                    let cp = 20_000 - Score::TB_WIN + score.abs();
                    format!("cp {}", if score.is_positive() { cp } else { -cp })
                }
                _ => {
                    let mate = (Score::MATE - score.abs() + score.is_positive() as i32) / 2;
                    format!("mate {}", if score.is_positive() { mate } else { -mate })
                }
            };

            if upperbound {
                formatted_score.push_str(" upperbound");
            } else if lowerbound {
                formatted_score.push_str(" lowerbound");
            }

            let mut line = format!(
                "info depth {depth} seldepth {} multipv {} score {formatted_score} nodes {} time {ms} nps {nps:.0} hashfull {} tbhits {} pv",
                root_move.sel_depth,
                pv_index + 1,
                self.shared.nodes.aggregate(),
                self.shared.tt.hashfull(),
                self.shared.tb_hits.aggregate(),
            );

            line.push_str(&format!(" {}", root_move.mv.to_uci(&self.board)));
            for mv in root_move.pv.line() {
                line.push_str(&format!(" {}", mv.to_uci(&self.board)));
            }

            self.writer.write_line(&line);
        }
    }

    fn print_uci_no_move(&mut self) {
        if self.board.in_check() {
            self.writer.write_line("info depth 0 score mate 0");
        } else {
            self.writer.write_line("info depth 0 score cp 0");
        }
    }
}

#[derive(Clone)]
pub struct RootMove {
    pub mv: Move,
    pub score: i32,
    pub previous_score: i32,
    pub display_score: i32,
    pub upperbound: bool,
    pub lowerbound: bool,
    pub sel_depth: i32,
    pub nodes: u64,
    pub pv: PrincipalVariationTable,
    pub tb_rank: i32,
    pub tb_score: i32,
}

impl Default for RootMove {
    fn default() -> Self {
        Self {
            mv: Move::NULL,
            score: -Score::INFINITE,
            previous_score: -Score::INFINITE,
            display_score: -Score::INFINITE,
            upperbound: false,
            lowerbound: false,
            sel_depth: 0,
            nodes: 0,
            pv: PrincipalVariationTable::default(),
            tb_rank: 0,
            tb_score: 0,
        }
    }
}

#[derive(Clone)]
pub struct PrincipalVariationTable {
    table: Box<[[Move; MAX_PLY + 1]]>,
    len: [usize; MAX_PLY + 1],
}

impl PrincipalVariationTable {
    pub fn line(&self) -> &[Move] {
        &self.table[0][..self.len[0]]
    }

    pub const fn clear(&mut self, ply: usize) {
        self.len[ply] = 0;
    }

    pub fn update(&mut self, ply: usize, mv: Move) {
        self.table[ply][0] = mv;
        self.len[ply] = self.len[ply + 1] + 1;

        for i in 0..self.len[ply + 1] {
            self.table[ply][i + 1] = self.table[ply + 1][i];
        }
    }

    pub fn commit_full_root_pv(&mut self, src: &Self, start_ply: usize) {
        let len = src.len[start_ply].min(MAX_PLY + 1);
        self.len[0] = len;
        self.table[0][..len].copy_from_slice(&src.table[start_ply][..len]);
    }
}

impl Default for PrincipalVariationTable {
    fn default() -> Self {
        Self {
            table: vec![[Move::NULL; MAX_PLY + 1]; MAX_PLY + 1].into_boxed_slice(),
            len: [0; MAX_PLY + 1],
        }
    }
}
