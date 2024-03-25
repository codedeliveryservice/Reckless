use super::{zobrist::ZOBRIST, Board};
use crate::types::{FullMove, Move, MoveKind, Piece, Square};

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    /// Updates the board representation by making a null move.
    pub fn make_null_move(&mut self) {
        self.ply += 1;
        self.side_to_move = !self.side_to_move;
        self.move_stack.push(FullMove::NULL);
        self.state_stack.push(self.state);

        self.state.hash ^= ZOBRIST.side;
        self.state.hash ^= ZOBRIST.castling[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }
    }

    /// Updates the board representation by making the specified `Move`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `Move` is illegal.
    pub fn make_move<const UPDATE_NNUE: bool>(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        let start = mv.start();
        let target = mv.target();
        let piece = self.piece_on(start);

        self.ply += 1;
        self.move_stack.push(FullMove::new(piece, mv));
        self.state_stack.push(self.state);

        if UPDATE_NNUE {
            self.nnue.push();
        }

        self.state.hash ^= ZOBRIST.side;
        self.state.hash ^= ZOBRIST.castling[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }

        if mv.is_capture() || piece == Piece::Pawn {
            self.state.halfmove_clock = 0;
        } else {
            self.state.halfmove_clock += 1;
        }

        if mv.is_capture() && !mv.is_en_passant() {
            self.remove_piece::<UPDATE_NNUE>(self.piece_on(target), !self.side_to_move, target);
        }

        self.remove_piece::<UPDATE_NNUE>(piece, self.side_to_move, start);
        self.add_piece::<UPDATE_NNUE>(piece, self.side_to_move, target);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = Square::new((start as u8 + target as u8) / 2);
                self.state.hash ^= ZOBRIST.en_passant[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece::<UPDATE_NNUE>(Piece::Pawn, !self.side_to_move, target ^ 8);
            }
            MoveKind::Castling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.remove_piece::<UPDATE_NNUE>(Piece::Rook, self.side_to_move, rook_start);
                self.add_piece::<UPDATE_NNUE>(Piece::Rook, self.side_to_move, rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece::<UPDATE_NNUE>(Piece::Pawn, self.side_to_move, target);
                self.add_piece::<UPDATE_NNUE>(mv.get_promotion_piece().unwrap(), self.side_to_move, target);
            }
            _ => (),
        }

        self.state.castling.update(start, target);
        self.state.hash ^= ZOBRIST.castling[self.state.castling];
        self.side_to_move = !self.side_to_move;

        let king = self.their(Piece::King).pop();
        if self.is_square_attacked(king, !self.side_to_move) {
            self.undo_move::<UPDATE_NNUE>();
            return Err(IllegalMoveError);
        }

        Ok(())
    }

    /// Restores the board representation to the state before the last move.
    ///
    /// # Panics
    ///
    /// Panics if the state stack is empty.
    pub fn undo_move<const UPDATE_NNUE: bool>(&mut self) {
        self.ply -= 1;
        self.side_to_move = !self.side_to_move;
        self.state = self.state_stack.pop().unwrap();
        self.move_stack.pop().unwrap();

        if UPDATE_NNUE {
            self.nnue.pop();
        }
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
