#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::if_same_then_else)]

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

#[allow(warnings)]
#[cfg(feature = "syzygy")]
mod bindings;

fn main() {
    lookup::initialize();
    nnue::initialize();

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(None),
        _ => uci::message_loop(),
    }
}
