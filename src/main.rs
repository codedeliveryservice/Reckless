#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::if_same_then_else)]
#![allow(unsafe_op_in_unsafe_fn)]

mod board;
mod evaluation;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod numa;
mod parameters;
mod search;
mod stack;
mod thread;
mod threadpool;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[cfg(feature = "syzygy")]
mod tb;

#[cfg(feature = "syzygy")]
#[allow(warnings)]
mod bindings;

fn main() {
    lookup::initialize();
    nnue::initialize();

    let buffer: std::collections::VecDeque<String> = std::env::args().skip(1).collect();

    match std::env::args().nth(1).as_deref() {
        Some("collect") => {
            let files: Vec<_> = std::env::args().skip(2).collect();
            tools::collect_buckets(&files);
        }
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

            tools::convert_pgns(&input, &output, threads);
        }
        _ => uci::message_loop(buffer),
    }
}
