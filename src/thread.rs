use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::{
    board::Board,
    history::{ContinuationHistory, CorrectionHistory, NoisyHistory, QuietHistory},
    stack::Stack,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::{is_loss, is_win, Move, Score, MAX_PLY},
};

pub struct ThreadPool<'a> {
    vector: Vec<ThreadData<'a>>,
}

impl<'a> ThreadPool<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool) -> Self {
        Self { vector: vec![ThreadData::new(tt, stop)] }
    }

    pub fn set_count(&mut self, threads: usize) {
        let tt = self.vector[0].tt;
        let stop = self.vector[0].stop;

        self.vector.resize_with(threads, || ThreadData::new(tt, stop));
    }

    pub fn main_thread(&mut self) -> &mut ThreadData<'a> {
        &mut self.vector[0]
    }

    pub fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ThreadData<'a>> {
        self.vector.iter_mut()
    }

    pub fn clear(&mut self) {
        for thread in &mut self.vector {
            *thread = ThreadData::new(thread.tt, thread.stop);
        }
    }
}

pub struct ThreadData<'a> {
    pub tt: &'a TranspositionTable,
    pub stop: &'a AtomicBool,
    pub board: Board,
    pub time_manager: TimeManager,
    pub stack: Stack,
    pub pv: PrincipalVariationTable,
    pub noisy_history: NoisyHistory,
    pub quiet_history: QuietHistory,
    pub continuation_history: ContinuationHistory,
    pub pawn_corrhist: CorrectionHistory,
    pub minor_corrhist: CorrectionHistory,
    pub major_corrhist: CorrectionHistory,
    pub non_pawn_corrhist: [CorrectionHistory; 2],
    pub last_move_corrhist: CorrectionHistory,
    pub node_table: NodeTable,
    pub lmr: LmrTable,
    pub stopped: bool,
    pub nodes: u64,
    pub root_depth: i32,
    pub completed_depth: i32,
    pub ply: usize,
}

impl<'a> ThreadData<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool) -> Self {
        Self {
            tt,
            stop,
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite, 0),
            stack: Stack::default(),
            pv: PrincipalVariationTable::default(),
            noisy_history: NoisyHistory::default(),
            quiet_history: QuietHistory::default(),
            continuation_history: ContinuationHistory::default(),
            pawn_corrhist: CorrectionHistory::default(),
            minor_corrhist: CorrectionHistory::default(),
            major_corrhist: CorrectionHistory::default(),
            non_pawn_corrhist: [CorrectionHistory::default(), CorrectionHistory::default()],
            last_move_corrhist: CorrectionHistory::default(),
            node_table: NodeTable::default(),
            lmr: LmrTable::default(),
            stopped: false,
            nodes: 0,
            root_depth: 0,
            completed_depth: 0,
            ply: 0,
        }
    }

    pub fn set_stop(&self, value: bool) {
        self.stop.store(value, Ordering::Relaxed);
    }

    pub fn get_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn conthist(&self, index: usize, mv: Move) -> i32 {
        if self.ply < index || self.stack[self.ply - index].mv == Move::NULL {
            return 0;
        }

        let prev_piece = self.stack[self.ply - index].piece;
        let prev_mv = self.stack[self.ply - index].mv;
        self.continuation_history.get(&self.board, prev_piece, prev_mv, mv)
    }

    pub fn print_uci_info(&self, depth: i32, score: i32, now: Instant) {
        let nps = self.nodes as f64 / now.elapsed().as_secs_f64();
        let ms = now.elapsed().as_millis();

        let score = match score {
            s if is_win(s) => format!("mate {}", (Score::MATE - score + 1) / 2),
            s if is_loss(s) => format!("mate {}", (-Score::MATE - score) / 2),
            _ => format!("cp {score}"),
        };

        print!(
            "info depth {depth} score {score} nodes {} time {ms} nps {nps:.0} hashfull {} pv",
            self.nodes,
            self.tt.hashfull(),
        );
        for mv in self.pv.line() {
            print!(" {mv}");
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
    table: [[i32; 64]; 64],
}

impl LmrTable {
    pub fn reduction(&self, depth: i32, move_count: i32) -> i32 {
        self.table[depth.min(63) as usize][move_count.min(63) as usize]
    }
}

impl Default for LmrTable {
    #[allow(clippy::needless_range_loop)]
    fn default() -> Self {
        let mut table = [[0; 64]; 64];

        for depth in 1..64 {
            for move_count in 1..64 {
                let reduction = 1000.0 + 455.0 * (depth as f32).ln() * (move_count as f32).ln();
                table[depth][move_count] = reduction as i32;
            }
        }

        Self { table }
    }
}

pub struct NodeTable {
    table: [[u64; 64]; 64],
}

impl NodeTable {
    pub fn add(&mut self, mv: Move, nodes: u64) {
        self.table[mv.from()][mv.to()] += nodes;
    }

    pub fn get(&self, mv: Move) -> u64 {
        self.table[mv.from()][mv.to()]
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for NodeTable {
    fn default() -> Self {
        Self { table: [[0; 64]; 64] }
    }
}
