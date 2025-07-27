#![allow(clippy::if_same_then_else)]

mod board;
mod evaluate;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod stack;
mod tb;
mod thread;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[allow(warnings)]
mod bindings;

fn main() {
    lookup::init();

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(None),
        Some("relabel") => {
            let input = std::env::args().nth(2).unwrap();
            let output = std::env::args().nth(3).unwrap();

            relabel(input, output);
        }
        _ => uci::message_loop(),
    }
}

fn relabel(input: String, output: String) {
    let input_file = File::open(input).unwrap();
    let output_file = File::create(output).unwrap();

    let mut reader = BinpackReader::new(BufReader::new(input_file));
    let mut writer = BinpackWriter::new(BufWriter::new(output_file));

    let mut network = Network::default();
    let mut count = 0;

    while let Some((position, entries)) = reader.next() {
        let mut board = position.to_board();

        if board.castling().is_allowed(CastlingKind::WhiteKingside) {
            let mut rook_from = Square::H1;
            while board.piece_on(rook_from) != Piece::new(Color::White, PieceType::Rook) {
                rook_from = rook_from.shift(-1);
            }
            board.castling_rooks[CastlingKind::WhiteKingside as usize] = rook_from;
        }

        if board.castling().is_allowed(CastlingKind::WhiteQueenside) {
            let mut rook_from = Square::A1;
            while board.piece_on(rook_from) != Piece::new(Color::White, PieceType::Rook) {
                rook_from = rook_from.shift(1);
            }
            board.castling_rooks[CastlingKind::WhiteQueenside as usize] = rook_from;
        }

        if board.castling().is_allowed(CastlingKind::BlackKingside) {
            let mut rook_from = Square::H8;
            while board.piece_on(rook_from) != Piece::new(Color::Black, PieceType::Rook) {
                rook_from = rook_from.shift(-1);
            }
            board.castling_rooks[CastlingKind::BlackKingside as usize] = rook_from;
        }

        if board.castling().is_allowed(CastlingKind::BlackQueenside) {
            let mut rook_from = Square::A8;
            while board.piece_on(rook_from) != Piece::new(Color::Black, PieceType::Rook) {
                rook_from = rook_from.shift(1);
            }
            board.castling_rooks[CastlingKind::BlackQueenside as usize] = rook_from;
        }

        let mut relabeled = vec![];
        for (mv, actual) in entries {
            let estimated = network.evaluate(&board) as i16;
            // let estimated = (0.7 * (estimated as f32)) as i16;

            // if !mv.is_capture() && !mv.is_promotion() && !board.in_check() && actual.abs() < 3000 {
            //     count += 1;
            // }

            relabeled.push((mv, estimated));
            board.make_move(mv);
        }

        writer.write(&position.to_board(), position.result, &relabeled);

        // if count > 10_000_000 {
        //     break;
        // }
    }
}

use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
};

use crate::{
    board::Board,
    nnue::Network,
    types::{Bitboard, Castling, CastlingKind, Color, Move, Piece, PieceType, Square},
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

    pub fn to_board(&self) -> Board {
        let mut board = Board {
            side_to_move: self.side_to_move,
            fullmove_number: self.fullmove_number as usize,
            ..Board::default()
        };

        board.state.en_passant = self.en_passant;
        board.state.castling = self.castling;
        board.state.halfmove_clock = self.halfmove_clock;

        for color in [Color::White, Color::Black] {
            for piece_type in 0..6 {
                let piece = Piece::new(color, PieceType::new(piece_type));

                for square in self.pieces[piece_type] & self.colors[color] {
                    board.add_piece(piece, square);
                }
            }
        }

        board
    }
}

pub struct BinpackWriter {
    buf: BufWriter<File>,
}

impl BinpackWriter {
    pub fn new(buf: BufWriter<File>) -> Self {
        Self { buf }
    }

    pub fn write(&mut self, board: &Board, result: u8, entries: &[(Move, i16)]) {
        // Header
        let length = (POSITION_SIZE + ENTRY_SIZE * entries.len()) as u32;
        self.buf.write_all(&length.to_le_bytes()).unwrap();

        // Position
        let position = Position::new(board, result);
        self.buf.write_all(as_bytes(&position)).unwrap();

        // Entries
        for entry in entries {
            self.buf.write_all(as_bytes(&entry.0)).unwrap();
            self.buf.write_all(as_bytes(&entry.1)).unwrap();
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

    pub fn next_bytes(&mut self) -> Option<Vec<u8>> {
        let mut header = [0; 4];
        if self.buf.read_exact(&mut header).is_err() {
            return None;
        }

        let length = u32::from_le_bytes(header) as usize;
        let mut data = vec![0; length];

        if self.buf.read_exact(&mut data).is_err() {
            return None;
        }

        Some(data)
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
