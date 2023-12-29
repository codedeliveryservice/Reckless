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
    uci::message_loop();
}
