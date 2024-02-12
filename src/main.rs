use std::path::PathBuf;

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
    let mut args = std::env::args();
    if let Some("datagen") = args.nth(1).as_deref() {
        let mut output = PathBuf::from("data");
        let mut threads = 1;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--output" => output = args.next().unwrap().into(),
                "--threads" => threads = args.next().unwrap().parse().unwrap(),
                _ => {}
            }
        }

        return tools::datagen(output, threads);
    }

    uci::message_loop();
}
