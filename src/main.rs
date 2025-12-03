#![allow(clippy::if_same_then_else)]

mod board;
mod evaluation;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod stack;
mod tb;
mod thread;
mod threadpool;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[allow(warnings)]
mod bindings;

fn main() {
    lookup::init();
    nnue::initialize();

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(None),
        Some("duplicates") => {
            let files: Vec<_> = std::env::args().skip(2).collect();
            tools::duplicates(&files);
        }
        Some("stats") => {
            let files: Vec<_> = std::env::args().skip(2).collect();
            tools::stats(&files);
        }
        Some("convert") => {
            let input = std::env::args().nth(2).unwrap();
            let output = std::env::args().nth(3).unwrap();
            let threads = std::env::args().nth(4).unwrap().parse().unwrap();
            let adversarial = std::env::args().nth(5).unwrap().parse().unwrap();

            tools::convert_pgns(&input, &output, threads, adversarial);
        }
        _ => uci::message_loop(),
    }
}
