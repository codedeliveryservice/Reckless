use std::{
    fs::File,
    io::{BufWriter, Write},
};

use crate::{
    board::Board,
    search::SearchResult,
    types::{Bitboard, Castling, Color, Move, Square},
};

pub const POSITION_SIZE: usize = std::mem::size_of::<Position>();
pub const ENTRY_SIZE: usize = std::mem::size_of::<(Move, i16)>();

const _: () = assert!(POSITION_SIZE == 72);
const _: () = assert!(ENTRY_SIZE == 4);

#[repr(C)]
pub struct Position {
    pub pieces: [Bitboard; 6], // 48 bytes
    pub colors: [Bitboard; 2], // 16 bytes
    pub side_to_move: Color,   // 1 byte
    pub en_passant: Square,    // 1 byte
    pub castling: Castling,    // 1 byte
    pub halfmove_clock: u8,    // 1 byte
    pub fullmove_number: u16,  // 2 bytes
    pub result: u8,            // 1 byte
    pub extra: [u8; 1],        // 1 byte
}

impl Position {
    pub fn new(board: &Board, result: u8) -> Self {
        Self {
            pieces: board.pieces,
            colors: board.colors,
            side_to_move: board.side_to_move,
            en_passant: board.state.en_passant,
            castling: board.state.castling,
            halfmove_clock: board.state.halfmove_clock,
            fullmove_number: board.fullmove_number as u16,
            result,
            extra: [0],
        }
    }
}

pub struct BinpackWriter {
    buf: BufWriter<File>,
}

impl BinpackWriter {
    pub fn new(buf: BufWriter<File>) -> Self {
        Self { buf }
    }

    pub fn write(&mut self, board: &Board, result: u8, entries: &[SearchResult]) {
        // Header
        let length = (POSITION_SIZE + ENTRY_SIZE * entries.len()) as u32;
        self.buf.write_all(&length.to_le_bytes()).unwrap();

        // Position
        let position = Position::new(board, result);
        self.buf.write_all(as_bytes(&position)).unwrap();

        // Entries
        for entry in entries {
            self.buf.write_all(as_bytes(&entry.best_move)).unwrap();
            self.buf.write_all(as_bytes(&(entry.score as i16))).unwrap();
        }
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        // Header
        let length = data.len() as u32;
        self.buf.write_all(&length.to_le_bytes()).unwrap();

        // Data
        self.buf.write_all(data).unwrap();
    }
}

fn as_bytes<T>(data: &T) -> &[u8] {
    let pointer = data as *const _ as *const u8;
    unsafe { std::slice::from_raw_parts(pointer, std::mem::size_of::<T>()) }
}
