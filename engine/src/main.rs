mod engine;
mod perft;
mod uci;

use engine::Engine;

fn main() {
    let mut engine = Engine::new();

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        uci::execute(&mut engine, buffer);
    }
}
