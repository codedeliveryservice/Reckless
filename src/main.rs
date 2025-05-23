#![allow(dead_code)]
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

fn main() {
    lookup::init();

    if let Some("bench") = std::env::args().nth(1).as_deref() {
        tools::bench::<false>(None);
        return;
    }

    if std::env::args().any(|v| v.contains("genfens")) {
        tools::genfens();
        return;
    }

    uci::message_loop();
}
