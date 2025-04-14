#![allow(clippy::if_same_then_else)]
mod board;
mod evaluate;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod stack;
mod thread;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

fn main() {
    lookup::init();

    #[cfg(feature = "datagen")]
    datagen(std::env::args());

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(14),
        _ => uci::message_loop(),
    }

    misc::dbg_print();
}

#[cfg(feature = "datagen")]
fn datagen(mut args: std::env::Args) {
    let command = args.nth(1);

    if let Some("datagen") = command.as_deref() {
        const USAGE: &str = "Usage: datagen <output> <book> <threads>";

        let output = args.next().expect(USAGE);
        let book = args.next().expect(USAGE);
        let threads = args.next().and_then(|v| v.parse().ok()).expect(USAGE);

        tools::datagen(output, book, threads);
        return;
    }

    if let Some("convert") = command.as_deref() {
        use tools::datagen::bulletformat::*;

        const USAGE: &str = "Usage: convert <input> <output> [options]";

        let input = args.next().expect(USAGE);
        let output = args.next().expect(USAGE);

        let mut filter = Filter { min_ply: 24, max_ply: 320, max_score: 2800 };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--min-ply" => filter.min_ply = args.next().unwrap().parse().unwrap(),
                "--max-ply" => filter.max_ply = args.next().unwrap().parse().unwrap(),
                "--max-score" => filter.max_score = args.next().unwrap().parse().unwrap(),
                _ => panic!("Unknown argument: {arg}"),
            }
        }

        convert_to_bullet_format(&input, &output, filter);
        std::process::exit(0);
    }
}
