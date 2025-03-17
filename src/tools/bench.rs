//! Bench is primarily used for testing the engine to verify that a change
//! does not introduce functional issues and that the engine's behavior
//! remains consistent. This is considered assuming that the change is not
//! activated by very rare conditions or only activated at a higher depth
//! than specified.
//!
//! Note that although it can be used as a benchmarking tool,
//! it is not comprehensive enough to be definitive.

use std::{sync::atomic::AtomicBool, time::Instant};

use crate::{
    board::Board,
    search::{self, Report},
    thread::ThreadData,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
};

const POSITIONS: &[&str] = &[
    "2k5/2P3p1/3r1p2/7p/2RB2rP/3K2P1/5P2/8 w - - 1 48",
    "8/8/1k1NK3/r7/2R2P1P/3n2P1/8/8 b - - 0 59",
    "r1r3k1/1bqnbp1N/ppn1p1p1/4P1B1/8/2N5/PPB1QPPP/R3R1K1 w - - 3 9",
    "2kr1b1r/pp1qpp2/1np4p/3n1Pp1/3PN2B/2N2Q2/1PP3PP/2KRR3 w - g6 0 12",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "6k1/8/pB2p1p1/3p3p/7q/PB6/Q7/2R3K1 w - - 0 37",
    "8/8/1p2k2P/1P6/P3B3/4B1K1/1b3P2/4n3 w - - 7 77",
    "7r/p1r1p2p/2k5/1p2n1N1/1Pp5/2R1P1P1/P4P1P/3R1K2 w - - 2 31",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "5rk1/1p2r1p1/p4np1/P2p4/2pNbP2/2P1PB2/6PP/3RR1K1 w - - 2 23",
    "8/3k4/3b3p/R1p1p2P/p1Pr4/P2P1r2/2K1R3/4B3 b - - 4 75",
    "rn1qr1k1/1b3n1p/p5p1/1p3p2/2p1P3/2P2N1P/PPBNQPP1/R4RK1 w - - 2 9",
    "r7/1ppq1pkp/1b1p1p2/p4P2/1P2R3/P2Q1NP1/2P2P1P/6K1 w - - 0 13",
    "8/R3bpk1/4p3/3pPn1P/3P2K1/1rP4P/4N3/2B5 b - - 3 50",
    "2kn3r/ppp1b2p/4qp2/2P3p1/8/1Q3NPP/PP2PPK1/R1B5 w - - 0 10",
    "1r6/p1q2kpp/2p2b2/5p2/3p1P2/BP1P2P1/P1P2Q1P/4R1K1 b - - 10 20",
    "3r1bk1/p4pp1/Pq4bp/1B2p3/1P2N3/1N5P/5PP1/1QB3K1 w - - 1 29",
    "8/1R3pk1/3rpb1p/1P1p1p2/1P1P4/r4N1P/2R3PK/8 b - - 2 27",
    "6k1/8/1p2R3/1Pb1p1B1/8/1r5P/6K1/8 w - - 4 45",
    "r4rk1/1pp1qppp/pnnbp3/7b/3PP3/1PN1BNPP/P4PB1/R2Q1RK1 w - - 2 13",
    "8/p2r2pk/1p6/3p2pP/7N/P1R5/2p1r3/5R1K w - - 0 50",
    "1r3r1k/p1pbb1pp/1p1p1q2/4pP2/2P1P3/1P1P2P1/PB5P/R2Q1RK1 b - - 0 11",
    "1r1q2k1/3r2p1/p4p2/5p2/2p4Q/P3B3/1bP3PP/3R1RK1 w - - 0 19",
    "2k4r/1pp1b2p/p1n2p2/2P3p1/8/1P2BNPP/1P2PPK1/2R5 w - - 1 13",
    "8/1p6/3p1k2/Pp1Pp1bn/1P2P2p/3K3P/4NB2/8 w - - 9 38",
    "r2qk2r/1bpnppbp/3p2p1/3P4/PN2P3/4BP2/1P1QN1PP/R3K2R b KQkq - 0 6",
    "8/5k2/4p3/2Np4/3P4/3K1P2/7b/8 w - - 69 92",
    "2r1kb1r/pp2pp1p/2nN1n2/3p2B1/3P2b1/4PN2/q3BPPP/1R1QK2R b Kk - 1 4",
    "1r4k1/1q3bp1/6r1/pp1pPpB1/2p1nP2/P1P1QB1P/1P4RK/R7 w - - 1 38",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    "r2q1rk1/1bpnbppp/1p2p3/8/p2PN3/2P2N2/PP1Q1PPP/1B1RR1K1 b - - 1 14",
    "r4rk1/2qbbppp/p1n1p3/8/2p2P2/P1N1BNQ1/1PP3PP/3R1RK1 b - - 5 11",
    "r4rk1/1bq1ppbp/6p1/2p5/P3P3/4BP2/1P1QN1PP/R2R2K1 w - - 2 11",
    "8/5pk1/4pr2/8/3Q1P2/7K/8/8 b - - 76 136",
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "8/1k4p1/p4p1p/n1B5/2P3P1/7P/4KP2/8 w - - 0 41",
    "8/2q2p1k/1p2p1p1/4P1P1/p7/P4Q2/1p3PK1/1R6 b - - 3 38",
    "2r5/1pqn1pbk/p2p1np1/P2Pp1Bp/NPr1P3/6PP/3Q1PB1/1RR3K1 b - - 6 12",
    "6k1/5pbp/R5p1/Pp6/8/1r2P2P/6B1/6K1 w - - 0 31",
    "1r2qrk1/2p1bpp1/3p4/1pP2b1p/1P2N1nP/4P1P1/1BQ2PB1/3R2KR w - - 3 22",
    "2q1r1k1/1p2npbp/p5p1/8/8/2P2N1P/P2B1PP1/2QR2K1 w - - 2 17",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "8/8/1k1r2p1/p1p2nPp/P3RN1P/8/4KP2/8 b - - 13 55",
    "5r1k/1p3p1p/2p2p2/1q2bP1Q/3p1P2/1PP1R1P1/6KP/2N5 w - - 0 25",
    "8/8/5pk1/5Nn1/R3r1P1/8/6K1/8 w - - 4 65",
];

pub fn bench<const PRETTY: bool>(depth: i32) {
    if PRETTY {
        println!("{}", "-".repeat(50));
        println!("{:>15} {:>13} {:>15}", "Nodes", "Elapsed", "NPS");
        println!("{}", "-".repeat(50));
    }

    let time = Instant::now();

    let mut nodes = 0;
    let mut index = 0;

    for position in POSITIONS {
        let now = Instant::now();

        let tt = TranspositionTable::default();
        let stop = AtomicBool::new(false);

        let mut td = ThreadData::new(&tt, &stop);
        td.board = Board::new(position).unwrap();
        td.time_manager = TimeManager::new(Limits::Depth(depth), 0);

        search::start(&mut td, Report::None);

        nodes += td.nodes;
        index += 1;

        let seconds = now.elapsed().as_secs_f64();
        let knps = td.nodes as f64 / seconds / 1000.0;

        if PRETTY {
            println!("{index:>3} {:>11} {seconds:>12.3}s {knps:>15.3} kN/s", td.nodes);
        }
    }

    let seconds = time.elapsed().as_secs_f64();
    let knps = nodes as f64 / seconds / 1000.0;

    if PRETTY {
        println!("{}", "-".repeat(50));
        println!("{nodes:>15} {seconds:>12.3}s {knps:>15.3} kN/s");
        println!("{}", "-".repeat(50));
    } else {
        let nps = nodes as f64 / seconds;
        println!("Bench: {nodes} nodes {nps:.0} nps");
    }
}
