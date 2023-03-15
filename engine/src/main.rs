use engine::Engine;
use uci::{Parser, UciCommand};

mod engine;
mod uci;

fn main() {
    let mut engine = Engine::new();

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();

        if let Ok(command) = Parser::new(&buffer).parse_command() {
            if command == UciCommand::Quit {
                break;
            }

            engine.execute(command);
        }
    }
}
