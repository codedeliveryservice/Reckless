use std::sync::atomic::{AtomicU64, Ordering};

pub struct NodeCounter<'a> {
    buffer: u64,
    local: u64,
    global: &'a AtomicU64,
}

impl<'a> NodeCounter<'a> {
    pub const fn new(global: &'a AtomicU64) -> Self {
        Self { buffer: 0, local: 0, global }
    }

    pub fn inc(&mut self) {
        const BUFFER_SIZE: u64 = 2048;

        self.buffer += 1;
        if self.buffer >= BUFFER_SIZE {
            self.flush();
        }
    }

    pub const fn local(&self) -> u64 {
        self.local + self.buffer
    }

    pub fn global(&self) -> u64 {
        self.global.load(Ordering::Relaxed) + self.buffer
    }

    fn flush(&mut self) {
        self.local += self.buffer;
        self.global.fetch_add(self.buffer, Ordering::Relaxed);
        self.buffer = 0;
    }
}
