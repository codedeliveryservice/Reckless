use engine::Engine;

mod engine;
mod perft;
mod uci;

fn main() {
    let mut engine = Engine::new();
    engine.set_position(Engine::START_FEN);

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();

        if input.starts_with("quit") {
            break;
        }

        uci::execute_command(&mut engine, &input);
    }
}
