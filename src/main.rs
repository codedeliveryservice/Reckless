#![allow(dead_code, unused_variables)]

mod board;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod thread;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

fn main() {
    #[cfg(feature = "datagen")]
    datagen(std::env::args());

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(10),
        _ => uci::message_loop(),
    }

    misc::dbg_print();
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
