use crate::{
    board::Board,
    types::{Color, Piece},
};

const POSITION_SIZE: usize = size_of::<Position>();

#[allow(clippy::assertions_on_constants)]
const _: () = assert!(POSITION_SIZE == 32, "Position size is not 32 bytes");

// The fields are never read, but they're used to marshal the position into a byte array.
#[allow(dead_code)]
pub struct Position {
    /// A bitboard with 1s in the squares that are occupied by any piece.    
    occupancies: u64,
    /// Each piece is represented by 4 bits (32 pieces * 4 bits = 128 bits).
    ///
    /// The order of the pieces corresponds to the order of the squares
    /// in the `occupancies` bitboard.
    ///
    /// - The top bit represents the piece color (0 = white, 1 = black).
    /// - The bottom 3 bits represent the piece type (0 = pawn, 1 = knight, etc.).
    pieces: u128,
    score: i16,
    wdl: f32,
}

struct Occupancy {
    color: u8,
    piece: u8,
    square: u8,
}

impl Position {
    /// Creates a new training position.
    ///
    ///  # Arguments
    ///
    /// * `board` - The board to parse.
    /// * `score` - The perspective score of the position in favor of the side to move.
    /// * `wdl` - The result of the position (1.0 for white win, 0.5 for draw, 0.0 for black win).
    pub fn parse(board: &Board, score: i32, wdl: f32) -> Self {
        let reverse = board.side_to_move() == Color::Black;

        let mut packed = Vec::new();
        for color in [Color::White, Color::Black] {
            for piece in 0..6 {
                for square in board.of(Piece::new(piece), color) {
                    packed.push(Occupancy {
                        piece: piece as u8,
                        color: color as u8 ^ reverse as u8,
                        square: square as u8 ^ (reverse as u8 * 56),
                    });
                }
            }
        }

        packed.sort_by_key(|occ| occ.square as usize);

        let mut occupancies = 0;
        let mut pieces = 0;

        for (index, Occupancy { color, piece, square }) in packed.into_iter().enumerate() {
            pieces |= ((color as u128) << 3 | (piece as u128)) << (index * 4);
            occupancies |= 1 << square as usize;
        }

        Self {
            occupancies,
            pieces,
            score: score as i16,
            wdl: if reverse { 1.0 - wdl } else { wdl },
        }
    }

    /// Marshals the position into a byte array.
    pub fn as_bytes(&self) -> &[u8] {
        let pointer = self as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(pointer, POSITION_SIZE) }
    }
}
