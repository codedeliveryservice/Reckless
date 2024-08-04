mod board;
mod lookup;
mod nnue;
mod search;
mod tables;
mod time;
mod tools;
mod types;
mod uci;

fn main() {
    #[cfg(feature = "datagen")]
    datagen(std::env::args());

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(12),
        _ => uci::message_loop(),
    }
}

#[cfg(feature = "datagen")]
fn datagen(mut args: std::env::Args) {
    const USAGE: &str = "Usage: datagen <output> <threads>";

    if let Some("datagen") = args.nth(1).as_deref() {
        let output = args.next().expect(USAGE);
        let threads = args.next().and_then(|v| v.parse().ok()).expect(USAGE);

        tools::datagen(output, threads)
    } else {
        println!("The 'datagen' feature is enabled, but no arguments were provided.");
        println!("{USAGE}");
    }
}
