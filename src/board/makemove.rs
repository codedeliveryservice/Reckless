use super::{Board, CASTLING_KEYS, EN_PASSANT_KEYS, SIDE_KEY};
use crate::types::{Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making a null move.
    pub fn make_null_move(&mut self) {
        self.ply += 1;
        self.side_to_move = !self.side_to_move;
        self.move_stack.push(Move::NULL);
        self.state_stack.push(self.state);

        self.state.hash ^= SIDE_KEY;
        self.state.hash ^= CASTLING_KEYS[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            self.state.en_passant = Square::None;
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
        self.state.hash ^= CASTLING_KEYS[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            self.state.en_passant = Square::None;
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
            self.remove_piece(piece, !self.side_to_move, target);
        }

        self.remove_piece(piece, self.side_to_move, start);
        self.add_piece(piece, self.side_to_move, target);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = ((start as u8 + target as u8) / 2).into();
                self.state.hash ^= EN_PASSANT_KEYS[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece(Piece::Pawn, !self.side_to_move, target ^ 8);
            }
            MoveKind::Castling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.remove_piece(Piece::Rook, self.side_to_move, rook_start);
                self.add_piece(Piece::Rook, self.side_to_move, rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece(Piece::Pawn, self.side_to_move, target);
                self.add_piece(mv.get_promotion_piece().unwrap(), self.side_to_move, target);
            }
            _ => (),
        }

        self.state.castling.update(start, target);
        self.state.hash ^= CASTLING_KEYS[self.state.castling];
        self.side_to_move = !self.side_to_move;

        let king = self.their(Piece::King).pop();
        if self.is_square_attacked(king, !self.side_to_move) {
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
        self.side_to_move = !self.side_to_move;
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
