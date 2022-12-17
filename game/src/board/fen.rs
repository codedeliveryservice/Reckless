use crate::core::{Color, Piece, Square};

use super::Board;

#[derive(Debug)]
pub enum ParseFenError {
    InvalidNumberOfSections { length: usize },
    UnexpectedTurnColor { color: String },
    UnexpectedPiece { piece: char },
}

/// Implements interaction with the Forsyth–Edwards notation which is a standard way for describing
/// a particular board position of a chess game.
///
/// See [Wikipedia article](https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation) for more information.
pub struct Fen;

impl Fen {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is not valid.
    pub(crate) fn parse(fen: &str) -> Result<Board, ParseFenError> {
        InnerFen::default().parse(fen)
    }
}

#[derive(Default)]
struct InnerFen {
    board: Board,
}

impl InnerFen {
    const SEPARATOR: char = '/';

    pub fn parse(mut self, fen: &str) -> Result<Board, ParseFenError> {
        let parts: Vec<&str> = fen.split_whitespace().collect();

        if parts.len() != 6 {
            return Err(ParseFenError::InvalidNumberOfSections {
                length: parts.len(),
            });
        }

        self.set_pieces(parts[0])?;
        self.set_turn(parts[1])?;

        Ok(self.board)
    }

    fn set_pieces(&mut self, text: &str) -> Result<(), ParseFenError> {
        let mut rank = 7;
        let mut file = 0;

        for c in text.chars() {
            if c == Self::SEPARATOR {
                rank -= 1;
                file = 0;
            } else if let Some(skip) = c.to_digit(10) {
                file += skip;
            } else {
                let piece = self.parse_piece(c)?;
                let color = self.parse_color(c);
                let square = Square::from_axes(rank, file);

                self.board.add_piece(piece, color, square);

                file += 1;
            }
        }

        Ok(())
    }

    fn parse_piece(&self, c: char) -> Result<Piece, ParseFenError> {
        match c {
            'P' | 'p' => Ok(Piece::Pawn),
            'N' | 'n' => Ok(Piece::Knight),
            'B' | 'b' => Ok(Piece::Bishop),
            'R' | 'r' => Ok(Piece::Rook),
            'Q' | 'q' => Ok(Piece::Queen),
            'K' | 'k' => Ok(Piece::King),
            _ => Err(ParseFenError::UnexpectedPiece { piece: c }),
        }
    }

    fn parse_color(&self, c: char) -> Color {
        match c.is_uppercase() {
            true => Color::White,
            false => Color::Black,
        }
    }

    fn set_turn(&mut self, text: &str) -> Result<(), ParseFenError> {
        self.board.turn = match text {
            "w" => Ok(Color::White),
            "b" => Ok(Color::Black),
            _ => Err(ParseFenError::UnexpectedTurnColor {
                color: text.to_string(),
            }),
        }?;

        Ok(())
    }
}
