use std::sync::Once;

use super::Board;
use crate::lookup;

static LUT_INITIALIZED: Once = Once::new();

fn prepare_lut() {
    LUT_INITIALIZED.call_once(|| lookup::init());
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

        if board.is_legal(mv) {
            board.make_move(mv);
            nodes += if depth > 1 { perft(board, depth - 1) } else { 1 };
            board.undo_move(mv);
        }
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
