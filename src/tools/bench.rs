//! Bench is primarily used for testing the engine to verify that a change
//! does not introduce functional issues and that the engine's behavior
//! remains consistent. This is considered assuming that the change is not
//! activated by very rare conditions or only activated at a higher depth
//! than specified.
//!
//! Note that `bench` is by no means intended for comprehensive benchmarking of
//! performance-related assessments.

use std::{sync::Arc, time::Instant};

use crate::{board::Board, search::Searcher, timeman::Limits};

static BENCH: &str = include_str!("../../data/bench.txt");

/// Runs a fixed depth search on the bench positions.
pub fn bench(depth: usize) {
    let positions = BENCH.split('\n').collect::<Vec<_>>();

    println!("{}", "-".repeat(50));
    println!("{:>15} {:>13} {:>15}", "Nodes", "Elapsed", "NPS");
    println!("{}", "-".repeat(50));

    let time = Instant::now();

    let mut nodes = 0;
    let mut index = 0;

    for position in positions {
        let now = Instant::now();

        let mut search = Searcher::new(Board::new(position), Limits::FixedDepth(depth), Arc::default(), Arc::default());
        search.print_to_stdout = false;
        search.iterative_deepening();

        nodes += search.nodes;
        index += 1;

        let seconds = now.elapsed().as_secs_f32();
        let knps = search.nodes as f32 / seconds / 1000f32;

        println!("{index:>3} {:>11} {seconds:>12.3}s {knps:>15.3} kN/s", search.nodes);
    }

    let seconds = time.elapsed().as_secs_f32();
    let knps = nodes as f32 / seconds / 1000f32;

    println!("{}", "-".repeat(50));
    println!("{nodes:>15} {seconds:>12.3}s {knps:>15.3} kN/s");
    println!("{}", "-".repeat(50));
}
