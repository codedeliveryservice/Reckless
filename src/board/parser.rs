use std::str::FromStr;

use super::Board;
use crate::types::{Color, Square};

#[derive(Debug)]
pub enum ParseFenError {
    /// The FEN string is missing piece placement data.
    MissingPlacementData,
    /// The FEN string contains an invalid piece type character.
    InvalidPieceType,
    /// The FEN string contains an invalid active color.
    InvalidActiveColor,
}

impl FromStr for Board {
    type Err = ParseFenError;

    /// Parses a [Forsyth–Edwards Notation][fen] string into a `Board`.
    ///
    /// The parser is not very strict and will accept some invalid FEN strings,
    /// it's the responsibility of the GUI to ensure the FEN string is valid.
    ///
    /// [fen]: https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation
    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let mut board = Board::default();
        let mut parts = fen.split_whitespace();

        let rows = parts.next().ok_or(ParseFenError::MissingPlacementData)?.split('/');

        for (rank, row) in rows.rev().enumerate() {
            let mut file = 0;

            for symbol in row.chars() {
                if let Some(skip) = symbol.to_digit(10) {
                    file += skip as u8;
                    continue;
                }

                let piece = symbol.try_into().map_err(|()| ParseFenError::InvalidPieceType)?;
                let color = if symbol.is_uppercase() { Color::White } else { Color::Black };
                let square = Square::from_rank_file(rank as u8, file);

                board.add_piece::<true>(piece, color, square);
                file += 1;
            }
        }

        board.side_to_move = match parts.next() {
            Some("w") => Color::White,
            Some("b") => Color::Black,
            _ => return Err(ParseFenError::InvalidActiveColor),
        };

        board.state.castling = parts.next().unwrap_or_default().into();
        board.state.en_passant = parts.next().unwrap_or_default().try_into().unwrap_or_default();
        board.state.halfmove_clock = parts.next().unwrap_or_default().parse().unwrap_or_default();
        board.state.hash = board.generate_hash_key();

        Ok(board)
    }
}
