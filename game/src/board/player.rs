use crate::{Board, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making a null move.
    pub fn make_null_move(&mut self) {
        self.history.push(self.state);
        self.repetitions.push(self.hash);

        self.hash.update_side();
        self.hash.update_castling(self.state.castling);
        self.hash.update_en_passant(self.state.en_passant);

        self.state.previous_move = None;
        self.state.en_passant = None;

        self.turn.reverse();
    }

    /// Restores the board to the previous state after the last null move made.
    pub fn undo_null_move(&mut self) {
        self.turn.reverse();
        self.state = self.history.pop();
        self.hash = self.repetitions.pop();
    }

    /// Updates the board representation by making the specified `Move`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `Move` is not allowed by the rules of chess.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        self.history.push(self.state);
        self.repetitions.push(self.hash);

        self.hash.update_side();
        self.hash.update_castling(self.state.castling);
        self.hash.update_en_passant(self.state.en_passant);

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

        let piece = self.get_piece(start).unwrap();
        self.remove_piece(piece, self.turn, start);

        if mv.is_promotion() {
            self.add_piece(mv.get_promotion_piece(), self.turn, target);
        } else {
            self.add_piece(piece, self.turn, target);
        }

        if mv.is_castling() {
            let (rook_start, rook_target) = get_rook_move(target);
            self.add_piece(Piece::Rook, self.turn, rook_target);
            self.remove_piece(Piece::Rook, self.turn, rook_start);
        }

        self.state.en_passant = match kind == MoveKind::DoublePush {
            true => {
                let square = (start + target) / 2;
                self.hash.update_en_passant_square(square);
                Some(square)
            }
            false => None,
        };

        self.state.castling.update_for_square(start);
        self.state.castling.update_for_square(target);
        self.hash.update_castling(self.state.castling);
        self.turn.reverse();

        // The move is considered illegal if it exposes the king to an attack after it has been made
        let king = self.their(Piece::King).pop().unwrap();
        if self.is_square_attacked(king, self.turn) {
            self.undo_move();
            return Err(IllegalMoveError);
        }

        Ok(())
    }

    /// Restores the board to the previous state after the last move made.
    ///
    /// # Panics
    ///
    /// Panics if there is no previous `Move` or the `Move` is not allowed for the current `Board`.
    pub fn undo_move(&mut self) {
        let mv = self.state.previous_move.unwrap();
        let capture = self.state.captured_piece;

        self.turn.reverse();
        self.state = self.history.pop();

        let start = mv.start();
        let target = mv.target();
        let kind = mv.kind();

        if mv.is_promotion() {
            self.add_piece(Piece::Pawn, self.turn, start);
            self.remove_piece(mv.get_promotion_piece(), self.turn, target);
        } else {
            let piece = self.get_piece(target).unwrap();
            self.add_piece(piece, self.turn, start);
            self.remove_piece(piece, self.turn, target);
        }

        if kind == MoveKind::EnPassant {
            let target = target.shift(-self.turn.offset());
            self.add_piece(Piece::Pawn, self.turn.opposite(), target);
        } else if mv.is_capture() {
            self.add_piece(capture.unwrap(), self.turn.opposite(), target);
        } else if mv.is_castling() {
            let (rook_start, rook_target) = get_rook_move(target);
            self.add_piece(Piece::Rook, self.turn, rook_start);
            self.remove_piece(Piece::Rook, self.turn, rook_target);
        }

        self.hash = self.repetitions.pop();
    }
}

fn get_rook_move(king_target: Square) -> (Square, Square) {
    match king_target {
        Square::G1 => (Square::H1, Square::F1),
        Square::C1 => (Square::A1, Square::D1),
        Square::G8 => (Square::H8, Square::F8),
        Square::C8 => (Square::A8, Square::D8),
        _ => panic!("Unexpected king target square '{}'", king_target),
    }
}
