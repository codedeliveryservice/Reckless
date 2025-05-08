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

    let args = std::env::args().collect::<Vec<_>>();

    if let Some("bench") = args.get(1).map(|s| s.as_str()) {
        let hash = args.get(2).and_then(|v| v.parse().ok());
        let threads = args.get(3).and_then(|v| v.parse().ok());
        let depth = args.get(4).and_then(|v| v.parse().ok());

        tools::bench::<false>(hash, threads, depth);
    } else {
        uci::message_loop();
    }
}

#[cfg(feature = "datagen")]
fn datagen(mut args: std::env::Args) {
    const USAGE: &str = "Usage: datagen <output> <book> <threads>";

    if let Some("datagen") = args.nth(1).as_deref() {
        let output = args.next().expect(USAGE);
        let book = args.next().expect(USAGE);
        let threads = args.next().and_then(|v| v.parse().ok()).expect(USAGE);

        tools::datagen(output, book, threads)
    } else {
        println!("The 'datagen' feature is enabled, but no arguments were provided.");
        println!("{USAGE}");
    }
}
