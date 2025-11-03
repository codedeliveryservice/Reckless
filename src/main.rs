#![allow(clippy::if_same_then_else)]

use std::sync::Arc;

use crate::{thread::SharedContext, tools::BenchOptions, uci::spawn_listener};

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

    match std::env::args().nth(1).as_deref() {
        Some("bench") => {
            let tokens = std::env::args().skip(2).collect::<Vec<_>>();
            tools::bench(BenchOptions::parse(tokens));
        }
        _ => {
            let shared = Arc::new(SharedContext::default());
            let rx = spawn_listener(shared.clone());

            uci::message_loop(rx, shared);
        }
    }
}
