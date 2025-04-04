use std::{
    fs::File,
    io::{BufWriter, Write},
};

use super::position::{Position, ENTRY_SIZE, POSITION_SIZE};
use crate::search::SearchResult;

pub struct BinpackWriter {
    buf: BufWriter<File>,
}

impl BinpackWriter {
    pub fn new(buf: BufWriter<File>) -> Self {
        Self { buf }
    }

    pub fn write(&mut self, position: Position, entries: &[SearchResult]) {
        // Header
        let length = (POSITION_SIZE + ENTRY_SIZE * entries.len()) as u32;
        self.buf.write_all(&length.to_le_bytes()).unwrap();

        // Position
        self.buf.write_all(as_bytes(&position)).unwrap();

        // Entries
        for entry in entries {
            self.buf.write_all(as_bytes(&entry.best_move)).unwrap();
            self.buf.write_all(as_bytes(&(entry.score as i16))).unwrap();
        }
    }
}

fn as_bytes<T>(data: &T) -> &[u8] {
    let pointer = data as *const _ as *const u8;
    unsafe { std::slice::from_raw_parts(pointer, std::mem::size_of::<T>()) }
}
