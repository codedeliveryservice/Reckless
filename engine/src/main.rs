mod commands;
mod engine;
mod parser;
mod perft;

use commands::UciCommand;
use engine::Engine;
use parser::Parser;

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
