use std::sync::Once;

use super::{Board, NullBoardObserver};
use crate::lookup;

static LUT_INITIALIZED: Once = Once::new();

fn prepare_lut() {
    LUT_INITIALIZED.call_once(|| lookup::initialize());
}

macro_rules! assert_perft {
    ($($name:ident: $fen:tt, [$($nodes:expr),*],)*) => {$(
        #[test]
        fn $name() {
            prepare_lut();

            let mut board = Board::from_fen($fen).unwrap();
            for (depth, &nodes) in [$($nodes),*].iter().enumerate() {
                assert_eq!(perft(&mut board, depth + 1), nodes);
            }
        }
    )*};
}

fn perft(board: &mut Board, depth: usize) -> u32 {
    let mut nodes = 0;
    for entry in board.generate_all_moves().iter() {
        let mv = entry.mv;

        board.make_move(mv, &mut NullBoardObserver);
        nodes += if depth > 1 { perft(board, depth - 1) } else { 1 };
        board.undo_move(mv);
    }
    nodes
}

// Test cases from https://www.chessprogramming.org/Perft_Results
assert_perft!(
    starting_position: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", [20, 400, 8902, 197281, 4865609],
    kiwipete: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", [48, 2039, 97862, 4085603],
    position_3: "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", [14, 191, 2812, 43238, 674624, 11030083],
    position_4: "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", [6, 264, 9467, 422333, 15833292],
    position_5: "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", [44, 1486, 62379, 2103487],
    position_6: "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", [46, 2079, 89890, 3894594],
);

// Test cases from https://www.chessprogramming.org/Chess960_Perft_Results
assert_perft!(
    chess960_1: "bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9", [21, 528, 12189, 326672, 8146062],
    chess960_2: "1nbbnrkr/p1p1ppp1/3p4/1p3P1p/3Pq2P/8/PPP1P1P1/QNBBNRKR w HFhf - 0 9", [28, 1120, 31058, 1171749, 34030312],
    chess960_3: "bqnr1kr1/pppppp1p/6p1/5n2/4B3/3N2PP/PbPPPP2/BQNR1KR1 w GDgd - 2 9", [31, 1132, 36559, 1261476, 43256823],
);

fn frc_board(fen: &str) -> Board {
    let mut board = Board::from_fen(fen).unwrap();
    board.set_frc(true); // Match UCI_Chess960 mode, in which to_fen emits Shredder castling.
    board
}

#[test]
fn to_fen_emits_shredder_castling_for_frc() {
    prepare_lut();

    // FRC positions whose rooks sit on non-standard files, so to_fen must emit
    // Shredder castling (rook-file letters) rather than KQkq. Fixtures reuse the
    // Chess960 perft positions above.
    for fen in [
        "bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9",
        "bqnr1kr1/pppppp1p/6p1/5n2/4B3/3N2PP/PbPPPP2/BQNR1KR1 w GDgd - 2 9",
    ] {
        assert_eq!(frc_board(fen).to_fen(), fen);
    }
}

fn assert_hash_consistent(board: &Board) {
    let mut recomputed = board.clone();
    recomputed.update_hash_keys();
    assert_eq!(board.hash(), recomputed.hash(), "incremental hash diverged from recomputation:\n{board}");
}

fn hash_perft(board: &mut Board, depth: usize) {
    assert_hash_consistent(board);
    if depth == 0 {
        return;
    }
    for entry in board.generate_all_moves().iter() {
        let mv = entry.mv;
        board.make_move(mv, &mut NullBoardObserver);
        hash_perft(board, depth - 1);
        board.undo_move(mv);
    }
}

#[test]
fn incremental_hash_matches_recomputation() {
    prepare_lut();

    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9",
    ] {
        let mut board = Board::from_fen(fen).unwrap();
        hash_perft(&mut board, 3);

        board.make_null_move();
        assert_hash_consistent(&board);
        board.undo_null_move();
    }
}
