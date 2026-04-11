//! Perft is used for testing, debugging, and benchmarking the move generator.
//! This is achieved by enumerating the number of leaf nodes at a given depth.
//!
//! See [Perft](https://www.chessprogramming.org/Perft) for more information.

use std::time::Instant;

use crate::{
    board::{Board, NullBoardObserver},
    types::{Move, MoveList},
};

pub fn perft(depth: usize, board: &mut Board) {
    println!("{}", "-".repeat(60));
    println!("{:>12} {:>12} {:>13} {:>15}", "Move", "Nodes", "Elapsed", "NPS");
    println!("{}", "-".repeat(60));

    let now = Instant::now();

    let mut nodes = 0;
    let mut index = 0;

    for entry in board.generate_all_moves().iter() {
        let now = Instant::now();

        let mv = entry.mv;

        board.make_move(mv, &mut NullBoardObserver);

        let count = perft_internal(&|board| board.generate_all_moves(), depth - 1, board);
        nodes += count;
        index += 1;

        board.undo_move(mv);

        let seconds = now.elapsed().as_secs_f64();
        let knps = count as f64 / seconds / 1000.0;

        println!("{index:>3} {:>8} {count:>12} {seconds:>12.3}s {knps:>15.3} kN/s", mv.to_uci(board));
    }

    let seconds = now.elapsed().as_secs_f64();
    let knps = nodes as f64 / seconds / 1000.0;

    println!("{}", "-".repeat(60));
    println!("{:>12} {nodes:>12} {seconds:>12.3}s {knps:>15.3} kN/s", "Total");
    println!("{}", "-".repeat(60));
}

pub fn simple_perft(depth: usize, board: &mut Board) {
    let mut nodes = 0;

    for entry in board.generate_all_moves().iter() {
        let mv = entry.mv;

        board.make_move(mv, &mut NullBoardObserver);

        let count = perft_internal(&|board| board.generate_all_moves(), depth - 1, board);
        nodes += count;

        board.undo_move(mv);

        println!("{}: {count}", mv.to_uci(board));
    }

    println!("total: {nodes}");
}

pub fn is_legal_perft(depth: usize, board: &mut Board) {
    let mut nodes = 0;

    for entry in is_legal_movegen(board).iter() {
        let mv = entry.mv;

        board.make_move(mv, &mut NullBoardObserver);

        let count = perft_internal(&is_legal_movegen, depth - 1, board);
        nodes += count;

        board.undo_move(mv);

        println!("{}: {count}", mv.to_uci(board));
    }

    println!("total: {nodes}");
}

fn perft_internal<F: Fn(&Board) -> MoveList>(move_gen: &F, depth: usize, board: &mut Board) -> u64 {
    if depth == 0 {
        return 1;
    }

    if depth == 1 {
        return move_gen(board).len() as u64;
    }

    let mut nodes = 0;

    for entry in move_gen(board).iter() {
        let mv = entry.mv;
        board.make_move(mv, &mut NullBoardObserver);
        nodes += perft_internal(move_gen, depth - 1, board);
        board.undo_move(mv);
    }

    nodes
}

fn is_legal_movegen(board: &Board) -> MoveList {
    let mut moves = MoveList::new();
    for i in 0..0x10000 {
        let j = i >> 12;
        if j == 0b0011 || j == 0b0110 || j == 0b0111 {
            continue;
        }
        let mv: Move = unsafe { std::mem::transmute(i as u16) };
        if mv.is_present() && board.is_legal(mv) {
            moves.push(mv.from(), mv.to(), mv.kind());
        }
    }
    moves
}
