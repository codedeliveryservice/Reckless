use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use crate::{
    board::Board, tables::{PrincipalVariationTable, TranspositionTable}, time::{Limits, TimeManager}, types::{is_loss, is_win, Score}
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

        self.vector.resize_with(threads, || ThreadData::new(&tt, stop));
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
            thread.clear();
        }
    }
}

pub struct ThreadData<'a> {
    pub tt: &'a TranspositionTable,
    pub stop: &'a AtomicBool,
    pub board: Board,
    pub time_manager: TimeManager,
    pub pv: PrincipalVariationTable,
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
            pv: PrincipalVariationTable::default(),
            stopped: false,
            nodes: 0,
            completed_depth: 0,
            ply: 0,
        }
    }

    pub fn set_stop(&self, value: bool) {
        self.stop.store(value, Ordering::Relaxed);
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
        for mv in &self.pv.variation() {
            print!(" {mv}");
        }
        println!();
    }

    pub fn clear(&mut self) {
        self.board = Board::starting_position();
    }
}
