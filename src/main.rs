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

#[macro_export]
macro_rules! time_it {
    ($name:expr, $block:block) => {{
        let start = std::time::Instant::now();
        let result = $block;
        let duration = start.elapsed();
        
        eprintln!("[{} #{}] '{}' took {:?}", file!(), line!(), $name, duration);
        result
    }};
}

fn main() {
    time_it!("lookup::init", {
    lookup::init();
    });

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(14),
        _ => uci::message_loop(),
    }
}
