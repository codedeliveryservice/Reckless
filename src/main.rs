use engine::Engine;

mod board;
mod cache;
mod engine;
mod evaluation;
mod lookup;
mod perft;
mod search;
mod tables;
mod timeman;
mod types;
mod uci;

fn main() {
    let mut engine = Engine::new();

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        uci::execute(&mut engine, buffer);
    }
}
