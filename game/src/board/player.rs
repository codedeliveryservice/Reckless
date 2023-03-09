use crate::{Board, Color, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making the specified `Move`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `Move` is not allowed by the rules of chess.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        self.history.push(self.state);

        self.repetitions.push(self.hash_key);

        self.hash_key.update_side();
        self.hash_key.update_castling(self.state.castling);
        self.hash_key.update_en_passant(self.state.en_passant);

        self.state.previous_move = Some(mv);

        let start = mv.start();
        let target = mv.target();
        let kind = mv.kind();

        if kind == MoveKind::EnPassant {
            let target = target.shift(-self.turn.offset());
            self.remove_piece(Piece::Pawn, self.turn.opposite(), target);
        } else if mv.is_capture() {
            let capture = self.get_piece(target).unwrap();
            self.remove_piece(capture, self.turn.opposite(), target);
            self.state.captured_piece = Some(capture);
        }

        if mv.is_promotion() {
            let piece = self.get_piece(start).unwrap();
            self.remove_piece(piece, self.turn, start);
            self.add_piece(mv.get_promotion_piece(), self.turn, target);
        } else {
            let piece = self.get_piece(start).unwrap();
            self.move_piece(piece, self.turn, start, target);
        }

        if kind == MoveKind::KingCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::H1, Square::F1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::H8, Square::F8),
            }
        } else if kind == MoveKind::QueenCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::A1, Square::D1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::A8, Square::D8),
            }
        }

        self.state.en_passant = match kind == MoveKind::DoublePush {
            true => {
                let square = Square((start.0 + target.0) / 2);
                self.hash_key.update_en_passant_square(square);
                Some(square)
            }
            false => None,
        };

        self.state.castling.update_for_square(start);
        self.state.castling.update_for_square(target);
        self.hash_key.update_castling(self.state.castling);
        self.turn.reverse();

        // The move is considered illegal if it exposes the king to an attack after it has been made
        let king = self.their(Piece::King).pop().unwrap();
        if self.is_square_attacked(king, self.turn) {
            self.take_back();
            return Err(IllegalMoveError);
        }

        Ok(())
    }

    /// Restores the board to the previous state after the last move made.
    ///
    /// # Panics
    ///
    /// Panics if there is no previous `Move` or the `Move` is not allowed for the current `Board`.
    pub fn take_back(&mut self) {
        self.hash_key.update_side();
        self.hash_key.update_castling(self.state.castling);
        self.hash_key.update_en_passant(self.state.en_passant);

        let mv = self.state.previous_move.unwrap();
        let capture = self.state.captured_piece;

        self.turn.reverse();
        self.state = self.history.pop();

        self.hash_key.update_castling(self.state.castling);
        self.hash_key.update_en_passant(self.state.en_passant);

        let start = mv.start();
        let target = mv.target();
        let kind = mv.kind();

        if mv.is_promotion() {
            self.remove_piece(mv.get_promotion_piece(), self.turn, target);
            self.add_piece(Piece::Pawn, self.turn, start);
        } else {
            let piece = self.get_piece(target).unwrap();
            self.move_piece(piece, self.turn, target, start);
        }

        if kind == MoveKind::EnPassant {
            let target = target.shift(-self.turn.offset());
            self.add_piece(Piece::Pawn, self.turn.opposite(), target);
        } else if mv.is_capture() {
            self.add_piece(capture.unwrap(), self.turn.opposite(), target);
        }

        if kind == MoveKind::KingCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::F1, Square::H1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::F8, Square::H8),
            }
        } else if kind == MoveKind::QueenCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::D1, Square::A1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::D8, Square::A8),
            }
        }

        self.hash_key = self.repetitions.pop();
    }
}
