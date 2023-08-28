use crate::core::{CastlingKind, Color, Square};

use super::Board;

type Error = Box<dyn std::error::Error>;

#[derive(Default)]
pub(crate) struct Fen {
    board: Board,
}

impl Fen {
    const SEPARATOR: char = '/';

    /// Returns the board corresponding to the specified Forsyth–Edwards notation which
    /// is a standard way for describing a particular board position of a chess game.
    ///
    /// See [Forsyth–Edwards notation](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation)
    /// for more information.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is not valid.
    pub fn parse(mut self, fen: &str) -> Result<Board, Error> {
        let parts: Vec<&str> = fen.split_whitespace().collect();

        if parts.len() != 6 {
            return Err(format!("Invalid number of sections: '{}'", parts.len()).into());
        }

        self.set_pieces(parts[0])?;
        self.set_turn(parts[1])?;
        self.set_castling(parts[2])?;
        self.set_en_passant(parts[3])?;

        self.board.hash = self.board.generate_hash_key();

        Ok(self.board)
    }

    fn set_pieces(&mut self, text: &str) -> Result<(), Error> {
        let mut rank = 7;
        let mut file = 0;

        for c in text.chars() {
            if c == Self::SEPARATOR {
                rank -= 1;
                file = 0;
            } else if let Some(skip) = c.to_digit(10) {
                file += skip as u8;
            } else {
                let piece = c.into();
                let color = self.parse_color(c);
                let square = Square::from_rank_file(rank, file);

                self.board.add_piece(piece, color, square);

                file += 1;
            }
        }

        Ok(())
    }

    fn parse_color(&self, c: char) -> Color {
        match c.is_uppercase() {
            true => Color::White,
            false => Color::Black,
        }
    }

    fn set_turn(&mut self, text: &str) -> Result<(), Error> {
        self.board.turn = match text {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(format!("Unexpected turn: '{}'", text).into()),
        };
        Ok(())
    }

    fn set_castling(&mut self, text: &str) -> Result<(), Error> {
        self.board.state.castling = text.into();
        Ok(())
    }

    fn set_en_passant(&mut self, text: &str) -> Result<(), Error> {
        if text == "-" {
            return Ok(());
        }

        if let Ok(square) = Square::try_from(text) {
            self.board.state.en_passant = Some(square);
            Ok(())
        } else {
            Err(format!("Unexpected en passant: '{}'", text).into())
        }
    }
}
