use super::{Board, BoardObserver};
use crate::types::{Move, MoveKind, Piece, PieceType, Square};

impl Board {
    fn increment_stack(&mut self) {
        self.halfmove_number += 1;
        self.state_stack.push(self.state);
        self.state.keys.toggle_side();
        self.state.keys.toggle_castling(self.state.castling);
        self.state.repetition = 0;

        if self.en_passant() != Square::None {
            self.state.keys.toggle_en_passant(self.en_passant());
            self.state.en_passant = Square::None;
        }
    }

    pub fn make_null_move(&mut self) {
        self.increment_stack();
        self.state.plies_from_null = 0;
        self.state.captured = None;
        self.update_threats();
    }

    pub fn undo_null_move(&mut self) {
        self.halfmove_number -= 1;
        self.state = self.state_stack.pop().unwrap();
    }

    /// Plays a move on the board and pushes the previous state onto the stack.
    ///
    /// This method assumes the move has been validated as legal per `Board::is_legal`.
    pub fn make_move<T: BoardObserver>(&mut self, mv: Move, observer: &mut T) {
        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_on(from);
        let stm = self.side_to_move();

        self.increment_stack();

        let captured = self.piece_on(to);
        self.state.captured = Some(captured);
        self.state.plies_from_null += 1;

        if mv.kind() == MoveKind::Capture || piece.piece_type() == PieceType::Pawn {
            self.state.fiftymove_clock = 0;
        } else {
            self.state.fiftymove_clock += 1;
        }

        if mv.is_castling() {
            let (rook_from, rook_to) = self.get_castling_rook(to);
            let rook = self.remove_piece(rook_from);
            observer.on_piece_change(self, rook, rook_from, false);

            self.remove_piece(from);
            self.add_piece(piece, to);
            observer.on_piece_move(self, piece, from, to);

            self.add_piece(rook, rook_to);
            observer.on_piece_change(self, rook, rook_to, true);
        } else if captured != Piece::None {
            self.remove_piece(from);
            observer.on_piece_change(self, piece, from, false);

            self.remove_piece(to);
            self.add_piece(piece, to);
            observer.on_piece_mutate(self, captured, piece, to);

            self.state.material -= captured.value();
            self.state.captured = Some(captured);
        } else {
            self.remove_piece(from);
            self.add_piece(piece, to);
            observer.on_piece_move(self, piece, from, to);

            if mv.is_en_passant() {
                let captured = self.remove_piece(to ^ 8);
                observer.on_piece_change(self, captured, to ^ 8, false);
                self.state.material -= captured.value();
                self.state.captured = Some(captured);
            } else if mv.is_double_push() {
                self.state.en_passant = to ^ 8;
                self.state.keys.toggle_en_passant(self.en_passant());
            }
        }

        if mv.is_promotion() {
            let promotion = Piece::new(stm, mv.promo_piece_type());

            self.remove_piece(to);
            self.add_piece(promotion, to);
            observer.on_piece_mutate(self, piece, promotion, to);

            self.state.material += promotion.value() - PieceType::Pawn.value();
        }

        self.state.castling.raw &= self.castling_rights[from] & self.castling_rights[to];
        self.state.keys.toggle_castling(self.state.castling);

        self.update_threats();
        self.validate_en_passant();

        let end = self.state.plies_from_null.min(self.fiftymove_clock() as usize);

        if end >= 4 {
            let mut idx = self.state_stack.len() as isize - 4;
            for i in (4..=end).step_by(2) {
                if idx < 0 {
                    break;
                }

                let stp = &self.state_stack[idx as usize];

                if stp.keys.full() == self.state.keys.full() {
                    self.state.repetition = if stp.repetition != 0 { -(i as i32) } else { i as i32 };
                    break;
                }

                idx -= 2;
            }
        }
    }

    pub fn undo_move(&mut self, mv: Move) {
        self.halfmove_number -= 1;

        let from = mv.from();
        let to = mv.to();
        let mover = self.remove_piece(to);
        let stm = self.side_to_move();

        if mv.is_castling() {
            let (rook_from, rook_to) = self.get_castling_rook(to);
            self.remove_piece(rook_to);
            self.add_piece(Piece::new(stm, PieceType::Rook), rook_from);
            self.add_piece(mover, from);
        } else {
            self.add_piece(if mv.is_promotion() { Piece::new(stm, PieceType::Pawn) } else { mover }, from);

            if mv.is_capture() {
                self.add_piece(self.captured_piece().expect("Empty capture."), mv.capture_sq());
            }
        }
        self.state = self.state_stack.pop().unwrap();
    }
}
