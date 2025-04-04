use crate::{
    board::Board,
    types::{Bitboard, Castling, Color, Move, Piece, PieceType, Square},
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
        let mut board = Board::default();

        board.side_to_move = self.side_to_move;
        board.fullmove_number = self.fullmove_number as usize;

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
