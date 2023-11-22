use super::{Board, CASTLING_KEYS, EN_PASSANT_KEYS, SIDE_KEY};
use crate::types::{Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making a null move.
    pub fn make_null_move(&mut self) {
        self.ply += 1;
        self.turn = !self.turn;
        self.move_stack.push(Move::default());
        self.state_stack.push(self.state);

        self.state.hash ^= SIDE_KEY;
        self.state.hash ^= CASTLING_KEYS[self.state.castling.0 as usize];

        if self.state.en_passant != Square::NO_SQUARE {
            self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            self.state.en_passant = Square::NO_SQUARE;
        }
    }

    /// Updates the board representation by making the specified `Move`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `Move` is illegal.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        self.ply += 1;
        self.move_stack.push(mv);
        self.state_stack.push(self.state);

        self.state.hash ^= SIDE_KEY;
        self.state.hash ^= CASTLING_KEYS[self.state.castling.0 as usize];

        if self.state.en_passant != Square::NO_SQUARE {
            self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            self.state.en_passant = Square::NO_SQUARE;
        }

        let start = mv.start();
        let target = mv.target();
        let piece = self.get_piece(start).unwrap();

        if mv.is_capture() || piece == Piece::Pawn {
            self.state.halfmove_clock = 0;
        } else {
            self.state.halfmove_clock += 1;
        }

        if let Some(piece) = self.get_piece(target) {
            self.remove_piece(piece, !self.turn, target);
        }

        self.remove_piece(piece, self.turn, start);
        self.add_piece(piece, self.turn, target);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = (start + target) / 2;
                self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece(Piece::Pawn, !self.turn, target ^ 8);
            }
            MoveKind::Castling => {
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
        self.state.hash ^= CASTLING_KEYS[self.state.castling.0 as usize];
        self.turn = !self.turn;

        let king = self.their(Piece::King).pop().unwrap();
        if self.is_square_attacked(king, !self.turn) {
            self.undo_move();
            return Err(IllegalMoveError);
        }

        Ok(())
    }

    /// Restores the board representation to the state before the last move.
    ///
    /// # Panics
    ///
    /// Panics if the state stack is empty.
    pub fn undo_move(&mut self) {
        self.ply -= 1;
        self.turn = !self.turn;
        self.state = self.state_stack.pop().unwrap();
        self.move_stack.pop().unwrap();
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
