mod bench;
mod perft;

#[cfg(feature = "datagen")]
mod datagen;

pub use bench::bench;
pub use perft::perft;

#[cfg(feature = "datagen")]
pub use datagen::datagen;
