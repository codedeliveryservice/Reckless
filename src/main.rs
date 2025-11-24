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
mod tb;
mod thread;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[allow(warnings)]
mod bindings;

#[allow(warnings)]
mod numa_bindings;

fn main() {
    lookup::init();

    assert!(
        unsafe { numa_bindings::numa_available() } != -1,
        "NUMA is not available on this system, but the binary was built with NUMA support."
    );

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(None),
        _ => uci::message_loop(),
    }
}
