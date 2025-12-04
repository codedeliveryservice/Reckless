#![allow(clippy::manual_is_multiple_of)]
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
        _ => uci::message_loop(),
    }
}
