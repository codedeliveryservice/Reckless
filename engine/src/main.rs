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

        match parser::parse_command(&buffer, engine.board.turn) {
            Ok(UciCommand::Quit) => break,
            Ok(command) => {
                if let Err(e) = engine.execute(command) {
                    eprintln!("{}", e);
                }
            }
            _ => eprintln!("Unknown command: '{}'", buffer.trim_end()),
        }
    }
}
