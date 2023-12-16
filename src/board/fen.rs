use super::Board;
use crate::types::{Color, Square};

/// Returns the FEN representation of the board.
///
/// See [Forsyth–Edwards notation](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation)
/// for more information.
pub fn from_fen(fen: &str) -> Board {
    let mut board = Board::default();
    let mut parts = fen.split_whitespace();

    for (rank, row) in parts.next().expect("Piece placement data").split('/').rev().enumerate() {
        let mut file = 0;

        for char in row.chars() {
            if let Some(skip) = char.to_digit(10) {
                file += skip as u8;
                continue;
            }

            let piece = char.into();
            let color = if char.is_uppercase() { Color::White } else { Color::Black };
            let square = Square::from_rank_file(rank as u8, file);

            board.add_piece(piece, color, square);
            file += 1;
        }
    }

    board.side_to_move = match parts.next().expect("Active color") {
        "w" => Color::White,
        "b" => Color::Black,
        _ => panic!("Invalid active color"),
    };

    board.state.castling = parts.next().expect("Castling availability").into();
    board.state.en_passant = parts.next().and_then(|square| square.try_into().ok()).unwrap_or(Square::None);
    board.state.halfmove_clock = parts.next().expect("Halfmove clock").parse().unwrap();
    board.state.hash = board.generate_hash_key();
    board
}
