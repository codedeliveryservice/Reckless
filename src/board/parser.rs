use super::Board;
use crate::{
    lookup::{between, ray_pass},
    types::{CastlingKind, Color, HOME_RANK, KING_TO_FILE, Piece, PieceType, ROOK_TO_FILE, Square},
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
                board.state.material += piece.value();
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
        board.update_en_passant();

        Ok(board)
    }

    fn set_castling(&mut self, rights: &str) {
        for right in rights.chars() {
            if !matches!(right.to_ascii_uppercase(), 'A'..='H' | 'K' | 'Q') {
                continue;
            }

            let color = if right.is_uppercase() { Color::White } else { Color::Black };
            let king_from = self.king_square(color);
            let mut search_step = right.to_ascii_uppercase() as i8 - b'A' as i8 - king_from.file() as i8;

            if right.eq_ignore_ascii_case(&'K') {
                search_step = Square::RIGHT;
            }
            if right.eq_ignore_ascii_case(&'Q') {
                search_step = Square::LEFT;
            }

            let rook_from =
                (ray_pass(king_from, king_from.shift(search_step)) & self.colored_pieces(color, PieceType::Rook)).lsb();

            let king_side = rook_from > king_from;

            let rights = if king_side { CastlingKind::KINGSIDE[color] } else { CastlingKind::QUEENSIDE[color] };

            let king_to =
                Square::from_rank_file(HOME_RANK[color].clone() as u8, KING_TO_FILE[king_side as usize].clone() as u8);
            let rook_to =
                Square::from_rank_file(HOME_RANK[color].clone() as u8, ROOK_TO_FILE[king_side as usize].clone() as u8);

            self.set_castling_for(rights, king_from, king_to, rook_from, rook_to);
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

    pub fn to_ascii(&self) -> String {
        let mut ascii = String::new();
        ascii.push_str("+---+---+---+---+---+---+---+---+\n");
        for rank in (0..8).rev() {
            ascii.push('|');
            for file in 0..8 {
                let square = Square::from_rank_file(rank, file);
                let piece = self.piece_on(square);
                let symbol = piece.try_into().unwrap_or(' ');
                ascii.push_str(&format!(" {symbol} |"));
            }
            ascii.push_str(&format!(" {}\n", rank + 1));
            ascii.push_str("+---+---+---+---+---+---+---+---+\n");
        }
        ascii.push_str("  a   b   c   d   e   f   g   h\n");
        ascii
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}\nFEN: {}", self.to_ascii(), self.to_fen())
    }
}
