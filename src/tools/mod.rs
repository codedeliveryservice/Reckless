mod bench;
mod perft;
mod speedtest;

pub use bench::bench;
pub use perft::is_legal_perft;
pub use perft::perft;
pub use perft::simple_perft;
pub use speedtest::speedtest;
