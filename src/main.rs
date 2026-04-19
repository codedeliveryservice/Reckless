#![allow(unsafe_op_in_unsafe_fn)]
#![warn(clippy::large_types_passed_by_value)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::redundant_clone)]

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
mod setwise;
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

    uci::message_loop(buffer);
}
