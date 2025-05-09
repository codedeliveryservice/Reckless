#![allow(clippy::if_same_then_else)]

mod board;
mod evaluate;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod stack;
mod thread;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[allow(warnings)]
mod bindings;

fn main() {
    lookup::init();

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(14),
        _ => uci::message_loop(),
    }
}
