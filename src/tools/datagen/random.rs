use std::{
    hash::{DefaultHasher, Hash, Hasher},
    time::SystemTime,
};

pub struct Random {
    pub seed: usize,
}

impl Random {
    fn splitmix64(mut x: usize) -> usize {
        x = x.wrapping_add(0x9E3779B97F4A7C15);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        x ^ (x >> 31)
    }

    pub fn new() -> Self {
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as usize;

        let mut hasher = DefaultHasher::new();
        std::thread::current().id().hash(&mut hasher);

        Self { seed: Self::splitmix64(time ^ hasher.finish() as usize) }
    }

    pub fn next(&mut self) -> usize {
        // https://en.wikipedia.org/wiki/Linear_congruential_generator
        self.seed = self.seed.wrapping_mul(0x5851F42D4C957F2D).wrapping_add(1);
        self.seed
    }
}
