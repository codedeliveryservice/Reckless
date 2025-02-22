use std::{
    ops::{Index, IndexMut},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::{
    board::Board,
    history::QuietHistory,
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
    pub quiet_history: QuietHistory,
    pub stopped: bool,
    pub nodes: u64,
    pub completed_depth: i32,
    pub ply: usize,
}

impl<'a> ThreadData<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool) -> Self {
        Self {
            tt,
            stop,
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite),
            stack: Stack::default(),
            pv: PrincipalVariationTable::default(),
            quiet_history: QuietHistory::default(),
            stopped: false,
            nodes: 0,
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

pub struct Stack {
    data: [StackEntry; MAX_PLY],
}

impl Default for Stack {
    fn default() -> Self {
        Self { data: [Default::default(); MAX_PLY] }
    }
}

#[derive(Copy, Clone)]
pub struct StackEntry {
    pub mv: Move,
}

impl Default for StackEntry {
    fn default() -> Self {
        Self { mv: Move::NULL }
    }
}

impl Index<usize> for Stack {
    type Output = StackEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for Stack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}
