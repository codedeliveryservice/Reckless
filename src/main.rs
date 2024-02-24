mod board;
mod cache;
mod lookup;
mod nnue;
mod search;
mod tables;
mod timeman;
mod tools;
mod types;
mod uci;

fn main() {
    if let Some("bench") = std::env::args().nth(1).as_deref() {
        tools::bench::<false>(10);
        return;
    }

    uci::message_loop();
}
