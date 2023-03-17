mod commands;
mod engine;
mod parser;
mod perft;

use commands::UciCommand;
use engine::Engine;

fn main() {
    let mut engine = Engine::new();

    loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();

        if let Ok(command) = parser::parse_command(&buffer) {
            if command == UciCommand::Quit {
                break;
            }

            engine.execute(command);
        }
    }
}
