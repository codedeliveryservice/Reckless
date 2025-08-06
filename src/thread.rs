use std::{
    ops::Index,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    board::Board,
    history::{ContinuationCorrectionHistory, ContinuationHistory, CorrectionHistory, NoisyHistory, QuietHistory},
    nnue::Network,
    stack::Stack,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::{normalize_to_cp, Move, Score, Square, MAX_MOVES, MAX_PLY},
};

pub struct ThreadPool<'a> {
    vector: Vec<ThreadData<'a>>,
}

impl<'a> ThreadPool<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool, nodes: &'a AtomicU64, tb_hits: &'a AtomicU64) -> Self {
        Self { vector: vec![ThreadData::new(tt, stop, nodes, tb_hits)] }
    }

    pub fn set_count(&mut self, threads: usize) {
        let tt = self.vector[0].tt;
        let stop = self.vector[0].stop;
        let nodes = self.vector[0].nodes.global;
        let tb_hits = self.vector[0].tb_hits.global;

        self.vector.resize_with(threads, || ThreadData::new(tt, stop, nodes, tb_hits));

        for i in 1..self.vector.len() {
            self.vector[i].board = self.vector[0].board.clone();
        }
    }

    pub fn main_thread(&mut self) -> &mut ThreadData<'a> {
        &mut self.vector[0]
    }

    pub fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ThreadData<'a>> {
        self.vector.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ThreadData<'a>> {
        self.vector.iter_mut()
    }

    pub fn clear(&mut self) {
        for thread in &mut self.vector {
            *thread = ThreadData::new(thread.tt, thread.stop, thread.nodes.global, thread.tb_hits.global);
        }
    }
}

impl<'a> Index<usize> for ThreadPool<'a> {
    type Output = ThreadData<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.vector[index]
    }
}

pub struct ThreadData<'a> {
    pub id: usize,
    pub tt: &'a TranspositionTable,
    pub stop: &'a AtomicBool,
    pub nodes: AtomicCounter<'a>,
    pub tb_hits: AtomicCounter<'a>,
    pub board: Board,
    pub time_manager: TimeManager,
    pub stack: Stack,
    pub nnue: Network,
    pub pv: PrincipalVariationTable,
    pub noisy_history: NoisyHistory,
    pub quiet_history: QuietHistory,
    pub continuation_history: ContinuationHistory,
    pub pawn_corrhist: CorrectionHistory,
    pub minor_corrhist: CorrectionHistory,
    pub major_corrhist: CorrectionHistory,
    pub non_pawn_corrhist: [CorrectionHistory; 2],
    pub continuation_corrhist: ContinuationCorrectionHistory,
    pub node_table: NodeTable,
    pub lmr: LmrTable,
    pub optimism: [i32; 2],
    pub stopped: bool,
    pub best_score: i32,
    pub root_depth: i32,
    pub root_delta: i32,
    pub sel_depth: i32,
    pub completed_depth: i32,
    pub ply: usize,
    pub nmp_min_ply: i32,
    pub previous_best_score: i32,
}

impl<'a> ThreadData<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool, nodes: &'a AtomicU64, tb_hits: &'a AtomicU64) -> Self {
        Self {
            id: 0,
            tt,            
            stop,
            nodes: AtomicCounter::new(nodes),
            tb_hits: AtomicCounter::new(tb_hits),
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite, 0, 0),
            stack: Stack::default(),
            nnue: Network::default(),
            pv: PrincipalVariationTable::default(),
            noisy_history: NoisyHistory::default(),
            quiet_history: QuietHistory::default(),
            continuation_history: ContinuationHistory::default(),
            pawn_corrhist: CorrectionHistory::default(),
            minor_corrhist: CorrectionHistory::default(),
            major_corrhist: CorrectionHistory::default(),
            non_pawn_corrhist: [CorrectionHistory::default(), CorrectionHistory::default()],
            continuation_corrhist: ContinuationCorrectionHistory::default(),
            node_table: NodeTable::default(),
            lmr: LmrTable::default(),
            optimism: [0; 2],
            stopped: false,
            best_score: -Score::INFINITE,
            root_depth: 0,
            root_delta: 0,
            sel_depth: 0,
            completed_depth: 0,
            ply: 0,
            nmp_min_ply: 0,
            previous_best_score: 0,
        }
    }

    pub fn get_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn conthist(&self, index: usize, mv: Move) -> i32 {
        if self.ply < index || self.stack[self.ply - index].mv.is_null() {
            return 0;
        }

        let piece = self.board.piece_on(mv.from());
        let sq = mv.to();
        self.continuation_history.get(self.stack[self.ply - index].conthist, piece, sq)
    }

    pub fn print_uci_info(&self, depth: i32, score: i32) {
        let elapsed = self.time_manager.elapsed();
        let nps = self.nodes.global() as f64 / elapsed.as_secs_f64();
        let ms = elapsed.as_millis();

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

        print!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {} tbhits {} pv",
            self.sel_depth,
            self.nodes.global(),
            self.tt.hashfull(),
            self.tb_hits.global(),
        );

        for mv in self.pv.line() {
            print!(" {}", mv.to_uci(&self.board));
        }

        if self.pv.line().is_empty() {
            print!(" {}", self.pv.best_move().to_uci(&self.board));
        }

        println!();
    }
}

pub struct PrincipalVariationTable {
    table: [[Move; MAX_PLY + 1]; MAX_PLY + 1],
    len: [usize; MAX_PLY + 1],
}

impl PrincipalVariationTable {
    pub const fn best_move(&self) -> Move {
        self.table[0][0]
    }

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
}

impl Default for PrincipalVariationTable {
    fn default() -> Self {
        Self {
            table: [[Move::NULL; MAX_PLY + 1]; MAX_PLY + 1],
            len: [0; MAX_PLY + 1],
        }
    }
}

pub struct LmrTable {
    table: [[i32; MAX_MOVES + 1]; MAX_MOVES + 1],
}

impl LmrTable {
    pub fn reduction(&self, depth: i32, move_count: i32) -> i32 {
        self.table[depth as usize][move_count as usize]
    }
}

impl Default for LmrTable {
    #[allow(clippy::needless_range_loop)]
    fn default() -> Self {
        let mut table = [[0; MAX_MOVES + 1]; MAX_MOVES + 1];

        for depth in 1..MAX_MOVES {
            for move_count in 1..MAX_MOVES {
                let reduction = 1000.0 + 455.0 * (depth as f32).ln() * (move_count as f32).ln();
                table[depth][move_count] = reduction as i32;
            }
        }

        Self { table }
    }
}

pub struct NodeTable {
    table: Box<[u64]>,
}

impl NodeTable {
    pub const fn add(&mut self, mv: Move, nodes: u64) {
        self.table[mv.encoded()] += nodes;
    }

    pub const fn get(&self, mv: Move) -> u64 {
        self.table[mv.encoded()]
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for NodeTable {
    fn default() -> Self {
        Self { table: vec![0; Square::NUM * Square::NUM].into_boxed_slice() }
    }
}

pub struct AtomicCounter<'a> {
    buffer: u64,
    local: u64,
    global: &'a AtomicU64,
}

impl<'a> AtomicCounter<'a> {
    pub const fn new(global: &'a AtomicU64) -> Self {
        Self { buffer: 0, local: 0, global }
    }

    pub const fn local(&self) -> u64 {
        self.local + self.buffer
    }

    pub fn global(&self) -> u64 {
        self.buffer + self.global.load(Ordering::Relaxed)
    }

    pub fn increment(&mut self) {
        const BUFFER_SIZE: u64 = 2048;

        self.buffer += 1;
        if self.buffer >= BUFFER_SIZE {
            self.flush();
        }
    }

    pub fn clear(&mut self) {
        self.local = 0;
        self.buffer = 0;
        self.global.store(0, Ordering::Relaxed);
    }

    pub fn flush(&mut self) {
        self.local += self.buffer;
        self.global.fetch_add(self.buffer, Ordering::Relaxed);
        self.buffer = 0;
    }
}
