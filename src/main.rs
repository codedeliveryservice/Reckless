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
    lookup::initialize();
    nnue::initialize();

    let buffer: std::collections::VecDeque<String> = std::env::args().skip(1).collect();

    uci::message_loop(buffer);
}
