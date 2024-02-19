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
    #[cfg(feature = "datagen")]
    datagen(std::env::args());

    uci::message_loop();
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
