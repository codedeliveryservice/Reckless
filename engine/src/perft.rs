//! Perft used for testing, debugging and benchmarking the move generator.
//! This is achieved by enumerating the number of leaf nodes for a given depth.
//!
//! See [Perft](https://www.chessprogramming.org/Perft) for more information.

use std::time::Instant;

use game::Board;

/// Runs a performance test on the `Board` with the specified depth.
pub fn run_perft(depth: usize, board: &mut Board) {
    println!("{}", "-".repeat(60));
    println!("{:>12} {:>12} {:>13} {:>15}", "Move", "Nodes", "Elapsed", "NPS");
    println!("{}", "-".repeat(60));

    let now = Instant::now();

    let mut nodes = 0;
    let mut index = 0;

    for mv in board.generate_moves() {
        let now = Instant::now();

        if board.make_move(mv).is_err() {
            continue;
        }

        let count = perft(depth - 1, board);
        nodes += count;
        index += 1;

        board.undo_move();

        let seconds = now.elapsed().as_secs_f32();
        let knps = count as f32 / seconds / 1000f32;

        println!("{:>3} {:>8} {:>12} {:>12.3}s {:>15.3} kN/s", index, mv, count, seconds, knps);
    }

    let seconds = now.elapsed().as_secs_f32();
    let knps = nodes as f32 / seconds / 1000f32;

    println!("{}", "-".repeat(60));
    println!("{:>12} {:>12} {:>12.3}s {:>15.3} kN/s", "Total", nodes, seconds, knps);
    println!("{}", "-".repeat(60));
}

#[inline(always)]
fn perft(depth: usize, board: &mut Board) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;

    for mv in board.generate_moves() {
        if board.make_move(mv).is_ok() {
            nodes += perft(depth - 1, board);
            board.undo_move();
        }
    }

    nodes
}
