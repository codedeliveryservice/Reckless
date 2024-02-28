//! Bench is primarily used for testing the engine to verify that a change
//! does not introduce functional issues and that the engine's behavior
//! remains consistent. This is considered assuming that the change is not
//! activated by very rare conditions or only activated at a higher depth
//! than specified.
//!
//! Note that `bench` is by no means intended for comprehensive benchmarking of
//! performance-related assessments.

use std::time::Instant;

use crate::{board::Board, cache::Cache, search::Searcher, tables::HistoryMoves, timeman::Limits};

const POSITIONS: &[&str] = &[
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
    "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    "rnbq1rk1/p3bppp/2p1pn2/1p6/2QP1B2/5NP1/PP2PPBP/RN2K2R w KQ - 0 9",
    "r1b1kb2/4pprN/pn4q1/2p1P2Q/8/5P2/PP4PP/R1B1R1K1 w q - 1 19",
    "6k1/p4p1p/1p2p3/5p1q/5P2/P3P3/1Q3PBP/R2r2K1 w - - 1 28",
    "r7/bppnqrpk/p2pp1np/P3p3/3PP3/2P1B1NP/1P1N1PP1/R2QR1K1 w - - 3 18",
    "5r1k/1pp3p1/3pp2p/1q6/1PnPPr2/1Q3N1P/2R2PPK/4R3 b - - 10 27",
    "8/1p6/p1pR4/4P1k1/6bp/8/4r1P1/4R1K1 w - - 4 67",
    "2r2rk1/5pp1/R3p3/3pPnBp/2Pn3P/1P1q4/3N1PP1/3QR1K1 w - - 1 25",
    "5r1k/1R4bp/8/3pp3/1P6/3rB2P/5PP1/4R1K1 w - - 0 33",
    "8/4n3/1p2kn2/4pp1p/1Pr1P2P/4PKN1/R4N2/8 w - - 4 56",
    "1b1r4/p5k1/2QqB2p/4N1p1/3r4/P7/5N1P/1R4K1 w - - 0 34",
    "2r5/pp2pk2/2n1bppp/8/4PP2/1PN2BP1/PK1R3P/8 b - - 1 24",
    "r2qb1k1/1pp2ppp/pnn5/8/3P4/1Q1B1N2/PP1N1PPP/4R1K1 b - - 1 20",
    "r1bq1rk1/1p1pnpbp/p1n1p1p1/2p5/2P1P3/P1NP2P1/1P2NPBP/R1BQK2R w KQ - 0 9",
    "3r2k1/5pqp/RbP5/1P1p4/2p1rQ2/8/5PPP/3N1BK1 w - - 3 33",
    "1r1r2k1/pp1b1pp1/1bnNp2p/2N5/8/P3P3/1P2BPPP/2RR2K1 b - - 4 19",
    "6k1/p4p2/1p4pp/2bB1q2/5P2/2P3P1/PP4PK/3Q4 w - - 0 27",
    "r2q1rk1/ppbn2pp/2p1p2n/3pPb2/P7/2P2N2/1P2BPPP/R1BQRNK1 w - - 1 13",
    "r2r4/pp3pk1/2b1p2p/5p2/2B5/4PP2/PP3KPP/2RR4 w - - 2 19",
    "rb1qk2r/1p1n1ppp/4bn2/pN1pp1B1/P2P4/1BP2N2/1P3PPP/R2QK2R b KQkq - 0 12",
    "5rk1/5qp1/p6p/8/1r4PP/1Nn1B3/1K1R4/Q5R1 b - - 2 30",
    "4q1k1/5pp1/2bP3p/4p2P/2B1Q3/6P1/5P2/6K1 w - - 3 38",
    "3r2k1/3N1p1p/2P1bnp1/4q3/1Q6/P4B1P/3p1PP1/3R2K1 b - - 2 30",
    "3rb1k1/4bpp1/4pn1p/1p2N3/p6P/5NQ1/PP3PP1/7K w - - 0 30",
    "8/1p1B1p2/p2b1kpp/3P4/8/5P1P/PP3PK1/8 b - - 2 33",
    "4r1k1/1pp2pp1/2n2n1p/1P2pq2/2N5/2PP1N1P/2Q2PP1/R5K1 b - - 0 23",
    "3r1k2/p1pB3r/1q3p2/4p2p/2n1P2B/2P2P2/P1P3PP/3QR2K w - - 2 27",
    "r4rk1/1pp3p1/p1nqb2p/4pp2/P1B5/1PQPP2P/2PN2P1/R4RK1 w - - 0 16",
    "r3k2r/pppqbppp/1nn1p1b1/4P3/3PB3/P1N1BP2/1P2N1PP/R2Q1RK1 b kq - 2 13",
    "q3k2r/1b2bppp/3p4/1p1Pp3/8/6P1/1PP2PBP/2BQK2R w Kk - 0 16",
    "r4rk1/1pp3pp/1nn1qp2/pQ2p3/4P3/1PP1PNNP/1P4P1/3R1RK1 w - - 1 20",
    "rn1qkb1r/1b3pp1/p2ppn1p/1p6/3NP1P1/P1N1BP2/1PPQ3P/R3KB1R b KQkq - 1 10",
    "1k6/ppp2p2/4p1bp/2P1P1p1/1n1B2P1/1B3P2/PP5P/6K1 b - - 0 30",
    "r1b1kb1r/1p1qpppp/p7/3n4/8/2N2N2/PP1P1PPP/R1BQK2R w KQkq - 0 9",
    "5rk1/5pp1/4p3/4PnBQ/4q2P/8/5PP1/3R2K1 b - - 6 34",
    "1Q6/pp2P1pk/2p4p/8/8/2P3K1/PP4P1/5qb1 b - - 3 35",
    "r1bqk2r/ppp2ppp/2n2P2/8/1bp5/2N2N2/PP3PPP/R1BQK2R w KQkq - 0 9",
    "8/4P1pk/p7/1p6/4R1Pp/7P/2r2r2/R3K3 b - - 1 54",
    "rn5r/1b2kppp/p3pn2/1pb5/8/1NN1P3/PP2BPPP/R1B2RK1 b - - 3 12",
    "6k1/5pp1/2Q1p2p/8/2N5/1P4P1/5PPK/3q4 b - - 2 34",
    "3r1rk1/ppb4p/4p3/3pn1pB/P1p5/2P3N1/1P2RPPP/3R2K1 w - - 0 23",
    "4k3/8/2P1Kp1p/p2B4/P4PP1/b7/8/8 w - - 29 66",
    "r4rk1/pppn1pp1/5n1p/1NbP4/2B3b1/5N2/PP3PPP/R1B1R1K1 w - - 4 17",
    "r1bq1rk1/pp1n1ppp/3bpn2/2p5/8/1QN2NP1/PP1PPPBP/R1B1K2R w KQ - 4 9",
    "r1b1kb1r/pp3ppp/2n1p1n1/1B1q4/3p4/2P2N2/PP3PPP/RNBQR1K1 w kq - 0 9",
    "1r4k1/5pb1/2Pqp1p1/1p5p/1P1p4/3P2PP/3Q1PBK/1R6 w - - 0 26",
    "r2r2k1/pp3p2/2n3pp/3Bb3/4P3/1PN3Pb/1P3P1P/R1BR2K1 b - - 2 19",
    "2r4k/2p3p1/1pQp3p/p1nNq3/P1P5/1P5P/2P3P1/5RK1 b - - 4 27",
    "r3kb1r/1b3ppp/p7/2p1q3/Pp6/4P3/RP2BPPP/2BQ1RK1 w kq - 0 15",
    "8/8/1p3kbp/3R4/1P1NPp2/7P/r5P1/6K1 w - - 5 41",
    "7k/6rp/3b2q1/3Ppp1p/1pP4Q/4RP2/4R1P1/7K w - - 0 43",
    "6k1/1p1r2p1/3Nn2p/3bP3/p7/4B2P/PP4P1/4R1K1 b - - 0 35",
    "r1q1kb1r/p2p1ppp/bpn1pn2/8/2Pp4/P4NP1/1P1NPPBP/R1BQ1RK1 w kq - 1 9",
    "1q6/5pkp/4n1p1/2bQp3/R3P3/6P1/5PKP/8 w - - 6 40",
    "3r2k1/5pp1/7p/5Q2/2n4q/1p3B1P/1P3PP1/3R2K1 b - - 0 37",
    "8/7p/6pk/3BQ3/8/7q/5P2/6K1 w - - 5 50",
];

/// Runs a fixed depth search on the bench positions.
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

        let mut board = Board::new(position).unwrap();
        let mut cache = Cache::default();
        let mut history = HistoryMoves::default();
        let mut search = Searcher::new(Limits::FixedDepth(depth), &mut board, &mut history, &mut cache);

        search.silent(true);
        search.run();

        nodes += search.nodes();
        index += 1;

        let seconds = now.elapsed().as_secs_f32();
        let knps = search.nodes() as f32 / seconds / 1000f32;

        if PRETTY {
            println!("{index:>3} {:>11} {seconds:>12.3}s {knps:>15.3} kN/s", search.nodes());
        }
    }

    let seconds = time.elapsed().as_secs_f32();
    let knps = nodes as f32 / seconds / 1000f32;

    if PRETTY {
        println!("{}", "-".repeat(50));
        println!("{nodes:>15} {seconds:>12.3}s {knps:>15.3} kN/s");
        println!("{}", "-".repeat(50));
    } else {
        let nps = nodes as f32 / seconds;
        println!("Bench: {nodes} nodes {nps:.0} nps");
    }
}
