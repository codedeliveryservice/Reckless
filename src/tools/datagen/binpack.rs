use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use super::position::{Position, ENTRY_SIZE, POSITION_SIZE};
use crate::{search::SearchResult, types::Move};

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

pub struct BinpackReader {
    buf: BufReader<File>,
}

impl BinpackReader {
    pub fn new(buf: BufReader<File>) -> Self {
        Self { buf }
    }

    pub fn next(&mut self) -> Option<(Position, Vec<(Move, i16)>)> {
        let mut header = [0; 4];
        if self.buf.read_exact(&mut header).is_err() {
            return None;
        }

        let length = u32::from_le_bytes(header) as usize;
        let mut data = vec![0; length];

        if self.buf.read_exact(&mut data).is_err() {
            return None;
        }

        let position = Self::deserialize_position(&data[..POSITION_SIZE]);
        let entries = Self::deserialize_entries(&data[POSITION_SIZE..]);

        Some((position, entries))
    }

    fn deserialize_position(bytes: &[u8]) -> Position {
        unsafe { std::ptr::read(bytes.as_ptr() as *const Position) }
    }

    fn deserialize_entries(bytes: &[u8]) -> Vec<(Move, i16)> {
        let (prefix, entries, suffix) = unsafe { bytes.align_to::<(Move, i16)>() };

        assert!(prefix.is_empty());
        assert!(suffix.is_empty());

        entries.to_vec()
    }
}

fn as_bytes<T>(data: &T) -> &[u8] {
    let pointer = data as *const _ as *const u8;
    unsafe { std::slice::from_raw_parts(pointer, std::mem::size_of::<T>()) }
}
