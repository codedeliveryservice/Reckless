mod bench;
mod perft;

pub use bench::bench;
pub use bench::profilebench;
pub use perft::is_legal_perft;
pub use perft::perft;
pub use perft::simple_perft;
