use super::Board;
use crate::{
    lookup::between,
    types::{CastlingKind, Color, Piece, PieceType, Square},
};

#[derive(Debug)]
pub enum ParseFenError {
    /// The FEN string is missing piece placement data.
    MissingPlacementData,
    /// The FEN string contains an invalid piece type character.
    InvalidPieceType,
    /// The FEN string contains an invalid active color.
    InvalidActiveColor,
}

impl Board {
    /// Parses a [Forsyth–Edwards Notation][fen] string into a `Board`.
    ///
    /// The parser is not very strict and will accept some invalid FEN strings,
    /// it's the responsibility of the GUI to ensure the FEN string is valid.
    ///
    /// [fen]: https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation
    pub fn from_fen(fen: &str) -> Result<Self, ParseFenError> {
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

        board.set_castling(parts.next().unwrap());

        board.state.en_passant = parts.next().unwrap_or_default().try_into().unwrap_or_default();
        board.state.halfmove_clock = parts.next().unwrap_or_default().parse().unwrap_or_default();
        board.fullmove_number = parts.next().unwrap_or_default().parse().unwrap_or_default();

        board.update_threats();
        board.update_king_threats();
        board.update_hash_keys();

        Ok(board)
    }

    fn set_castling(&mut self, rights: &str) {
        for right in rights.chars() {
            match right {
                'K' => {
                    let mut rook_from = Square::H1;
                    while self.piece_on(rook_from).piece_type() != PieceType::Rook {
                        rook_from = rook_from.shift(-1);
                    }

                    let king_from = self.king_square(Color::White);
                    self.set_castling_for(CastlingKind::WhiteKingside, king_from, Square::G1, rook_from, Square::F1);
                }
                'Q' => {
                    let mut rook_from = Square::A1;
                    while self.piece_on(rook_from).piece_type() != PieceType::Rook {
                        rook_from = rook_from.shift(1);
                    }

                    let king_from = self.king_square(Color::White);
                    self.set_castling_for(CastlingKind::WhiteQueenside, king_from, Square::C1, rook_from, Square::D1);
                }
                'k' => {
                    let mut rook_from = Square::H8;
                    while self.piece_on(rook_from).piece_type() != PieceType::Rook {
                        rook_from = rook_from.shift(-1);
                    }

                    let king_from = self.king_square(Color::Black);
                    self.set_castling_for(CastlingKind::BlackKingside, king_from, Square::G8, rook_from, Square::F8);
                }
                'q' => {
                    let mut rook_from = Square::A8;
                    while self.piece_on(rook_from).piece_type() != PieceType::Rook {
                        rook_from = rook_from.shift(1);
                    }

                    let king_from = self.king_square(Color::Black);
                    self.set_castling_for(CastlingKind::BlackQueenside, king_from, Square::C8, rook_from, Square::D8);
                }
                token @ 'A'..='H' => {
                    let king_from = self.king_square(Color::White);
                    let rook_from = Square::from_rank_file(0, token as u8 - b'A');

                    let kind = if king_from.file() < rook_from.file() {
                        CastlingKind::WhiteKingside
                    } else {
                        CastlingKind::WhiteQueenside
                    };

                    let (king_to, rook_to) = match kind {
                        CastlingKind::WhiteKingside => (Square::G1, Square::F1),
                        CastlingKind::WhiteQueenside => (Square::C1, Square::D1),
                        _ => unreachable!(),
                    };

                    self.set_castling_for(kind, king_from, king_to, rook_from, rook_to);
                }
                token @ 'a'..='h' => {
                    let king_from = self.king_square(Color::Black);
                    let rook_from = Square::from_rank_file(7, token as u8 - b'a');

                    let kind = if king_from.file() < rook_from.file() {
                        CastlingKind::BlackKingside
                    } else {
                        CastlingKind::BlackQueenside
                    };

                    let (king_to, rook_to) = match kind {
                        CastlingKind::BlackKingside => (Square::G8, Square::F8),
                        CastlingKind::BlackQueenside => (Square::C8, Square::D8),
                        _ => unreachable!(),
                    };

                    self.set_castling_for(kind, king_from, king_to, rook_from, rook_to);
                }
                _ => continue,
            }
        }
    }

    fn set_castling_for(
        &mut self, kind: CastlingKind, king_from: Square, king_to: Square, rook_from: Square, rook_to: Square,
    ) {
        self.state.castling.raw |= kind as u8;

        self.castling_rights[king_from] ^= kind as u8;
        self.castling_rights[rook_from] ^= kind as u8;

        self.castling_path[kind] |= between(king_from, king_to) | king_to.to_bb();
        self.castling_path[kind] |= between(rook_from, rook_to) | rook_to.to_bb();

        self.castling_path[kind] &= !king_from.to_bb();
        self.castling_path[kind] &= !rook_from.to_bb();

        self.castling_threat[kind] |= between(king_from, king_to) | king_from.to_bb() | king_to.to_bb();

        self.castling_rooks[kind] = rook_from;
    }

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
        fen.push_str(&self.state.castling.to_string(self));
        fen.push(' ');
        fen.push_str(&self.state.en_passant.to_string());
        fen.push(' ');
        fen.push_str(&self.state.halfmove_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());
        fen
    }
}
