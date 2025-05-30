use std::str::FromStr;

use super::Board;
use crate::types::{Color, Piece, Square};

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
        let mut board = Self::default();
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
                let square = Square::from_rank_file(rank as u8, file);

                board.add_piece(piece, square);
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
        board.fullmove_number = parts.next().unwrap_or_default().parse().unwrap_or_default();

        board.update_checkers();
        board.update_pinned();
        board.update_threats();
        board.update_hash_keys();

        Ok(board)
    }
}

impl Board {
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty_count = 0;

            for file in 0..8 {
                let piece = self.piece_on(Square::from_rank_file(rank, file));
                if piece == Piece::None {
                    empty_count += 1;
                    continue;
                }

                if empty_count > 0 {
                    fen.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                fen.push_str(&piece.to_string());
            }

            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }

            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push_str(&self.side_to_move.to_string());
        fen.push(' ');
        fen.push_str(&self.state.castling.to_string());
        fen.push(' ');
        fen.push_str(&self.state.en_passant.to_string());
        fen.push(' ');
        fen.push_str(&self.state.halfmove_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());
        fen
    }
}
