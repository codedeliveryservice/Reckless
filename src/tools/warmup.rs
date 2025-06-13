//! This tool plays a series of self-play games with predefined opening positions
//! to collect Profile-Guided Optimization (PGO) information during compilation.

use std::{
    sync::atomic::{AtomicBool, AtomicU64},
    time::Instant,
};

use crate::{
    board::Board,
    search::{self, Report},
    thread::ThreadData,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
};

const LIMITS: Limits = Limits::Nodes(64000);
const POSITIONS: &[&str] = &[
    "rnb1kbnr/ppp1pppp/8/4q3/8/2N5/PPPP1PPP/R1BQKBNR w KQkq - 2 4",
    "rnbqkbnr/pp2pppp/2p5/8/2pP4/4P3/PP3PPP/RNBQKBNR w KQkq - 0 4",
    "rnbqk1nr/pppp1ppp/3b4/4p3/3PP3/5N2/PPP2PPP/RNBQKB1R b KQkq - 0 3",
    "r1bqk2r/ppp1bppp/3p1n2/8/3QP3/2N5/PPP1BPPP/R1B1K2R w KQkq - 2 8",
];

pub fn warmup() {
    println!("Starting {} warmup games using limits: {LIMITS:?}", POSITIONS.len());

    let now = Instant::now();

    for position in POSITIONS {
        println!("Playing warmup game with position: {position}");
        play_game(position);
    }

    println!("Warmup completed in {} seconds", now.elapsed().as_secs());
}

fn play_game(position: &str) {
    let tt = TranspositionTable::default();
    let stop = AtomicBool::new(false);
    let counter = AtomicU64::new(0);
    let tb_hits = AtomicU64::new(0);

    let mut td = ThreadData::new(&tt, &stop, &counter, &tb_hits);

    td.board = Board::new(position).unwrap();
    td.time_manager = TimeManager::new(LIMITS, 0, 0);

    loop {
        if !td.board.has_legal_moves() || td.board.draw_by_repetition(0) || td.board.draw_by_fifty_move_rule() {
            break;
        }

        let result = search::start(&mut td, Report::None);
        td.board.make_move(result.best_move);
    }
}
