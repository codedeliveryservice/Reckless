use std::{
    ops::Index,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use crate::{
    board::Board,
    history::{ContinuationCorrectionHistory, ContinuationHistory, CorrectionHistory, NoisyHistory, QuietHistory},
    nnue::Network,
    stack::Stack,
    thread::pool::ScopeExt,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::{normalize_to_cp, Move, Score, MAX_MOVES, MAX_PLY},
};

#[repr(align(64))]
struct AlignedAtomicU64 {
    inner: AtomicU64,
}

pub struct Counter<const SIZE: usize> {
    shards: [AlignedAtomicU64; SIZE],
}

unsafe impl<const SIZE: usize> Sync for Counter<SIZE> {}

impl<const SIZE: usize> Counter<SIZE> {
    pub fn aggregate(&self) -> u64 {
        self.shards.iter().map(|shard| shard.inner.load(Ordering::Relaxed)).sum()
    }

    pub fn get(&self, id: usize) -> u64 {
        self.shards[id].inner.load(Ordering::Relaxed)
    }

    pub fn increment(&self, id: usize) {
        self.shards[id].inner.fetch_add(1, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        for shard in &self.shards {
            shard.inner.store(0, Ordering::Relaxed);
        }
    }
}

impl Default for Counter<{ SharedContext::MAX_THREADS }> {
    fn default() -> Self {
        Self {
            shards: std::array::from_fn(|_| AlignedAtomicU64 { inner: AtomicU64::default() }),
        }
    }
}

#[derive(Default)]
pub struct SharedContext {
    pub tt: TranspositionTable,
    pub stop: AtomicBool,
    pub nodes: Counter<{ Self::MAX_THREADS }>,
    pub tb_hits: Counter<{ Self::MAX_THREADS }>,
}

unsafe impl Send for SharedContext {}

impl SharedContext {
    const MAX_THREADS: usize = 512;
}

pub struct ThreadPool {
    pub workers: Vec<pool::WorkerThread>,
    pub vector: Vec<Box<ThreadData>>,
}

impl ThreadPool {
    pub fn new(shared: Arc<SharedContext>) -> Self {
        let workers = pool::make_worker_threads(1);
        let data = make_thread_data(shared, &workers);

        Self { workers, vector: data }
    }

    pub fn set_count(&mut self, threads: usize) {
        let shared = self.vector[0].shared.clone();

        self.workers.drain(..).for_each(pool::WorkerThread::join);
        self.workers = pool::make_worker_threads(threads);

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }

    pub fn main_thread(&mut self) -> &mut ThreadData {
        &mut self.vector[0]
    }

    pub fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Box<ThreadData>> {
        self.vector.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<ThreadData>> {
        self.vector.iter_mut()
    }

    pub fn clear(&mut self) {
        let shared = self.vector[0].shared.clone();

        std::mem::drop(self.vector.drain(..));
        self.vector = make_thread_data(shared, &self.workers);
    }
}

impl Index<usize> for ThreadPool {
    type Output = ThreadData;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vector[index]
    }
}

pub struct ThreadData {
    pub id: usize,
    pub shared: Arc<SharedContext>,
    pub board: Board,
    pub time_manager: TimeManager,
    pub stack: Stack,
    pub nnue: Network,
    pub root_moves: Vec<RootMove>,
    pub pv_table: PrincipalVariationTable,
    pub noisy_history: NoisyHistory,
    pub quiet_history: QuietHistory,
    pub continuation_history: ContinuationHistory,
    pub pawn_corrhist: CorrectionHistory,
    pub minor_corrhist: CorrectionHistory,
    pub major_corrhist: CorrectionHistory,
    pub non_pawn_corrhist: [CorrectionHistory; 2],
    pub continuation_corrhist: ContinuationCorrectionHistory,
    pub lmr: LmrTable,
    pub optimism: [i32; 2],
    pub stopped: bool,
    pub root_depth: i32,
    pub root_delta: i32,
    pub sel_depth: i32,
    pub completed_depth: i32,
    pub nmp_min_ply: i32,
    pub previous_best_score: i32,
    pub root_in_tb: bool,
    pub stop_probing_tb: bool,
}

impl ThreadData {
    pub fn new(shared: Arc<SharedContext>) -> Self {
        Self {
            id: 0,
            shared,
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite, 0, 0),
            stack: Stack::default(),
            nnue: Network::default(),
            root_moves: Vec::new(),
            pv_table: PrincipalVariationTable::default(),
            noisy_history: NoisyHistory::default(),
            quiet_history: QuietHistory::default(),
            continuation_history: ContinuationHistory::default(),
            pawn_corrhist: CorrectionHistory::default(),
            minor_corrhist: CorrectionHistory::default(),
            major_corrhist: CorrectionHistory::default(),
            non_pawn_corrhist: [CorrectionHistory::default(), CorrectionHistory::default()],
            continuation_corrhist: ContinuationCorrectionHistory::default(),
            lmr: LmrTable::default(),
            optimism: [0; 2],
            stopped: false,
            root_depth: 0,
            root_delta: 0,
            sel_depth: 0,
            completed_depth: 0,
            nmp_min_ply: 0,
            previous_best_score: 0,
            root_in_tb: false,
            stop_probing_tb: false,
        }
    }

    pub fn nodes(&self) -> u64 {
        self.shared.nodes.get(self.id)
    }

    pub fn get_stop(&self) -> bool {
        self.shared.stop.load(Ordering::Relaxed)
    }

    pub fn conthist(&self, ply: usize, index: usize, mv: Move) -> i32 {
        if ply < index || self.stack[ply - index].mv.is_null() {
            return 0;
        }

        let piece = self.board.piece_on(mv.from());
        let sq = mv.to();
        self.continuation_history.get(self.stack[ply - index].conthist, piece, sq)
    }

    pub fn print_uci_info(&self, depth: i32) {
        let elapsed = self.time_manager.elapsed();
        let nps = self.shared.nodes.aggregate() as f64 / elapsed.as_secs_f64();
        let ms = elapsed.as_millis();

        let root_move = &self.root_moves[0];
        let mut score = if root_move.score == -Score::INFINITE { root_move.display_score } else { root_move.score };

        let mut upperbound = root_move.upperbound;
        let mut lowerbound = root_move.lowerbound;

        if self.root_in_tb && score.abs() <= Score::TB_WIN {
            score = root_move.tb_score;
            upperbound = false;
            lowerbound = false;
        }

        let score = if score.abs() < Score::TB_WIN_IN_MAX {
            format!("cp {}", normalize_to_cp(score, &self.board))
        } else if score.abs() <= Score::TB_WIN {
            let ply = Score::TB_WIN - score.abs();
            let cp_score = 20_000 - ply;
            format!("cp {}", if score.is_positive() { cp_score } else { -cp_score })
        } else {
            let mate = (Score::MATE - score.abs() + if score.is_positive() { 1 } else { 0 }) / 2;
            format!("mate {}", if score.is_positive() { mate } else { -mate })
        };

        let score = if upperbound {
            format!("{score} upperbound")
        } else if lowerbound {
            format!("{score} lowerbound")
        } else {
            score
        };

        print!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {} tbhits {} pv",
            root_move.sel_depth,
            self.shared.nodes.aggregate(),
            self.shared.tt.hashfull(),
            self.shared.tb_hits.aggregate(),
        );

        print!(" {}", root_move.mv.to_uci(&self.board));

        for mv in root_move.pv.line() {
            print!(" {}", mv.to_uci(&self.board));
        }

        println!();
    }
}

#[derive(Clone)]
pub struct RootMove {
    pub mv: Move,
    pub score: i32,
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

    pub fn clear(&mut self, ply: usize) {
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

pub struct LmrTable {
    table: Box<[[i32; MAX_MOVES + 1]]>,
}

impl LmrTable {
    pub const fn reduction(&self, depth: i32, move_count: i32) -> i32 {
        self.table[depth as usize][move_count as usize]
    }
}

impl Default for LmrTable {
    fn default() -> Self {
        let mut table = vec![[0; MAX_MOVES + 1]; MAX_MOVES + 1].into_boxed_slice();

        for depth in 1..MAX_MOVES {
            for move_count in 1..MAX_MOVES {
                let reduction = 970.0027 + 457.7087 * (depth as f32).ln() * (move_count as f32).ln();
                table[depth][move_count] = reduction as i32;
            }
        }

        Self { table }
    }
}

pub fn make_thread_data(shared: Arc<SharedContext>, worker_threads: &[pool::WorkerThread]) -> Vec<Box<ThreadData>> {
    std::thread::scope(|scope| -> Vec<Box<ThreadData>> {
        let handles = worker_threads
            .iter()
            .map(|worker| {
                let (tx, rx) = std::sync::mpsc::channel();
                let shared = shared.clone();
                let join_handle = scope.spawn_into(
                    move || {
                        tx.send(Box::new(ThreadData::new(shared))).unwrap();
                    },
                    worker,
                );
                (rx, join_handle)
            })
            .collect::<Vec<_>>();

        let mut thread_data: Vec<Box<ThreadData>> = Vec::with_capacity(handles.len());
        for (rx, handle) in handles {
            let td = rx.recv().unwrap();
            thread_data.push(td);
            handle.join();
        }

        thread_data
    })
}

pub mod pool {
    use std::{
        sync::{
            mpsc::{Receiver, SyncSender},
            Arc, Condvar, Mutex,
        },
        thread::Scope,
    };

    // Handle for communicating with a worker thread.
    // Contains a sender for sending messages to the worker thread,
    // and a receiver for receiving messages from the worker thread.
    pub struct WorkSender {
        // INVARIANT: Each send must be matched by a receive.
        sender: SyncSender<Box<dyn FnOnce() + Send>>,
        completion_signal: Arc<(Mutex<bool>, Condvar)>,
    }

    /// Handle for the receiver side of a worker thread.
    struct WorkReceiver {
        receiver: Receiver<Box<dyn FnOnce() + Send>>,
        completion_signal: Arc<(Mutex<bool>, Condvar)>,
    }

    fn make_work_channel() -> (WorkSender, WorkReceiver) {
        let (sender, receiver) = std::sync::mpsc::sync_channel(0);
        let completion_signal = Arc::new((Mutex::new(false), Condvar::new()));

        (
            WorkSender { sender, completion_signal: Arc::clone(&completion_signal) },
            WorkReceiver { receiver, completion_signal },
        )
    }

    pub struct ReceiverHandle<'scope> {
        completion_signal: &'scope Arc<(Mutex<bool>, Condvar)>,
        received: bool,
    }

    impl ReceiverHandle<'_> {
        pub fn join(mut self) {
            let (lock, cvar) = &**self.completion_signal;
            let mut completed = lock.lock().unwrap();
            while !*completed {
                completed = cvar.wait(completed).unwrap();
            }
            drop(completed);
            self.received = true;
        }
    }

    impl Drop for ReceiverHandle<'_> {
        fn drop(&mut self) {
            // When the receiver handle is dropped, we ensure that we have received something.
            assert!(self.received, "ReceiverHandle was dropped without receiving a value");
        }
    }

    pub trait ScopeExt<'scope, 'env> {
        fn spawn_into<F>(&'scope self, f: F, comms: &'scope WorkerThread) -> ReceiverHandle<'scope>
        where
            F: FnOnce() + Send + 'scope;
    }

    impl<'scope, 'env> ScopeExt<'scope, 'env> for Scope<'scope, 'env> {
        fn spawn_into<'comms, F>(&'scope self, f: F, thread: &'scope WorkerThread) -> ReceiverHandle<'scope>
        where
            F: FnOnce() + Send + 'scope,
        {
            // Safety: This file is structured such that threads never hold the data longer than is permissible.
            let f = unsafe {
                std::mem::transmute::<Box<dyn FnOnce() + Send + 'scope>, Box<dyn FnOnce() + Send + 'static>>(Box::new(
                    f,
                ))
            };

            // Reset the completion flag before sending the task
            {
                let (lock, _) = &*thread.comms.completion_signal;
                let mut completed = lock.lock().unwrap();
                *completed = false;
            }

            thread.comms.sender.send(f).expect("Failed to send function to worker thread");

            ReceiverHandle {
                completion_signal: &thread.comms.completion_signal,
                // Important: We start with `received` as false.
                received: false,
            }
        }
    }

    fn make_worker_thread() -> WorkerThread {
        let (sender, receiver) = make_work_channel();

        let handle = std::thread::spawn(move || {
            while let Ok(work) = receiver.receiver.recv() {
                work();
                let (lock, cvar) = &*receiver.completion_signal;
                let mut completed = lock.lock().unwrap();
                *completed = true;
                drop(completed); // Release the lock before notifying
                cvar.notify_one();
            }
        });

        WorkerThread { handle, comms: sender }
    }

    pub fn make_worker_threads(num_threads: usize) -> Vec<WorkerThread> {
        (0..num_threads).map(|_| make_worker_thread()).collect()
    }

    pub struct WorkerThread {
        handle: std::thread::JoinHandle<()>,
        comms: WorkSender,
    }

    impl WorkerThread {
        pub fn join(self) {
            drop(self.comms); // Drop the sender to signal the worker thread to finish
            self.handle.join().expect("Worker thread panicked");
        }
    }
}
