mod board;
mod evaluate;
mod history;
mod lookup;
mod masks;
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
    masks::init();

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
