use crate::{Board, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making a null move.
    pub fn make_null_move(&mut self) {
        self.history.push(self.state, self.hash);

        self.hash.update_side();
        self.hash.update_castling(self.state.castling);
        self.hash.update_en_passant(self.state.en_passant);

        self.state.previous_move = Move::default();
        self.state.en_passant = None;

        self.turn.reverse();
    }

    /// Restores the board to the previous state after the last null move made.
    pub fn undo_null_move(&mut self) {
        self.turn.reverse();
        (self.state, self.hash) = self.history.pop();
    }

    /// Updates the board representation by making the specified `Move`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `Move` is not allowed by the rules of chess.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        self.history.push(self.state, self.hash);

        self.hash.update_side();
        self.hash.update_castling(self.state.castling);
        self.hash.update_en_passant(self.state.en_passant);

        self.state.previous_move = mv;
        self.state.captured_piece = None;
        self.state.en_passant = None;

        let start = mv.start();
        let target = mv.target();
        let piece = self.get_piece(start).unwrap();

        if let Some(piece) = self.get_piece(target) {
            self.remove_piece(piece, self.turn.opposite(), target);
            self.state.captured_piece = Some(piece);
        }

        self.remove_piece(piece, self.turn, start);
        self.add_piece(piece, self.turn, target);

        match mv.kind() {
            MoveKind::DoublePush => {
                let square = (start + target) / 2;
                self.hash.update_en_passant_square(square);
                self.state.en_passant = Some(square);
            }
            MoveKind::EnPassant => {
                self.remove_piece(Piece::Pawn, self.turn.opposite(), target ^ 8);
            }
            MoveKind::KingCastling | MoveKind::QueenCastling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.remove_piece(Piece::Rook, self.turn, rook_start);
                self.add_piece(Piece::Rook, self.turn, rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece(Piece::Pawn, self.turn, target);
                self.add_piece(mv.get_promotion_piece(), self.turn, target);
            }
            _ => (),
        }

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
        self.turn.reverse();

        let mv = self.state.previous_move;
        let start = mv.start();
        let target = mv.target();
        let piece = self.get_piece(target).unwrap();

        self.add_piece(piece, self.turn, start);
        self.remove_piece(piece, self.turn, target);

        if let Some(piece) = self.state.captured_piece {
            self.add_piece(piece, self.turn.opposite(), target);
        }

        match mv.kind() {
            MoveKind::EnPassant => {
                self.add_piece(Piece::Pawn, self.turn.opposite(), target ^ 8);
            }
            MoveKind::KingCastling | MoveKind::QueenCastling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.add_piece(Piece::Rook, self.turn, rook_start);
                self.remove_piece(Piece::Rook, self.turn, rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece(piece, self.turn, start);
                self.add_piece(Piece::Pawn, self.turn, start);
            }
            _ => (),
        }

        (self.state, self.hash) = self.history.pop();
    }
}

const fn get_rook_move(king_target: Square) -> (Square, Square) {
    match king_target {
        Square::G1 => (Square::H1, Square::F1),
        Square::C1 => (Square::A1, Square::D1),
        Square::G8 => (Square::H8, Square::F8),
        Square::C8 => (Square::A8, Square::D8),
        _ => panic!("Unexpected king target square"),
    }
}
