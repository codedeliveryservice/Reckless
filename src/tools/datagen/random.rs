use std::time::SystemTime;

pub struct Random {
    pub seed: usize,
}

impl Random {
    pub fn new() -> Self {
        Self {
            seed: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as usize,
        }
    }

    pub fn next(&mut self) -> usize {
        // https://en.wikipedia.org/wiki/Linear_congruential_generator
        self.seed = self.seed.wrapping_mul(0x5851F42D4C957F2D).wrapping_add(1);
        self.seed
    }
}
