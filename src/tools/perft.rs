//! Perft used for testing, debugging and benchmarking the move generator.
//! This is achieved by enumerating the number of leaf nodes for a given depth.
//!
//! See [Perft](https://www.chessprogramming.org/Perft) for more information.

use std::time::Instant;

use crate::board::Board;

/// Runs a performance test on the `Board` with the specified depth.
pub fn perft(depth: usize, board: &mut Board) {
    println!("{}", "-".repeat(60));
    println!("{:>12} {:>12} {:>13} {:>15}", "Move", "Nodes", "Elapsed", "NPS");
    println!("{}", "-".repeat(60));

    let now = Instant::now();

    let mut nodes = 0;
    let mut index = 0;

    for &mv in board.generate_all_moves().iter() {
        let now = Instant::now();

        if !board.make_move::<false>(mv) {
            board.undo_move::<false>();
            continue;
        }

        let count = perft_internal(depth - 1, board);
        nodes += count;
        index += 1;

        board.undo_move::<false>();

        let seconds = now.elapsed().as_secs_f64();
        let knps = count as f64 / seconds / 1000.0;

        println!("{index:>3} {mv:>8} {count:>12} {seconds:>12.3}s {knps:>15.3} kN/s");
    }

    let seconds = now.elapsed().as_secs_f64();
    let knps = nodes as f64 / seconds / 1000.0;

    println!("{}", "-".repeat(60));
    println!("{:>12} {nodes:>12} {seconds:>12.3}s {knps:>15.3} kN/s", "Total");
    println!("{}", "-".repeat(60));
}

fn perft_internal(depth: usize, board: &mut Board) -> u64 {
    let mut nodes = 0;

    for &mv in board.generate_all_moves().iter() {
        if !board.make_move::<false>(mv) {
            board.undo_move::<false>();
            continue;
        }

        nodes += if depth > 1 { perft_internal(depth - 1, board) } else { 1 };
        board.undo_move::<false>();
    }

    nodes
}
