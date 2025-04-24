use super::Board;
use crate::types::{Move, MoveKind, Piece, PieceType, Square, ZOBRIST};

impl Board {
    pub fn make_null_move(&mut self) {
        self.side_to_move = !self.side_to_move;
        self.state_stack.push(self.state);

        self.state.key ^= ZOBRIST.side;
        self.state.key ^= ZOBRIST.castling[self.state.castling];
        self.state.plies_from_null = 0;
        self.state.repetition = 0;

        self.update_threats();
        self.update_king_threats();

        if self.state.en_passant != Square::None {
            self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }
    }

    pub fn undo_null_move(&mut self) {
        self.side_to_move = !self.side_to_move;
        self.state = self.state_stack.pop().unwrap();
    }

    pub fn make_move(&mut self, mv: Move) {
        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_on(from);
        let pt = piece.piece_type();
        let stm = self.side_to_move;

        self.state_stack.push(self.state);

        self.state.key ^= ZOBRIST.side;
        self.state.key ^= ZOBRIST.castling[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }

        self.state.captured = None;

        if mv.kind() == MoveKind::Capture || pt == PieceType::Pawn {
            self.state.halfmove_clock = 0;
        } else {
            self.state.halfmove_clock += 1;
        }
        self.state.plies_from_null += 1;

        let captured = self.piece_on(to);
        if captured != Piece::None {
            self.remove_piece(captured, to);
            self.update_hash(captured, to);
            self.state.captured = Some(captured);
        }

        self.remove_piece(piece, from);
        self.add_piece(piece, to);

        self.update_hash(piece, from);
        self.update_hash(piece, to);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = Square::new((from as u8 + to as u8) / 2);
                self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                let captured = Piece::new(!stm, PieceType::Pawn);

                self.remove_piece(captured, to ^ 8);
                self.update_hash(captured, to ^ 8);
            }
            MoveKind::Castling => {
                let (rook_from, rook_to) = Self::get_castling_rook(to);
                let rook = Piece::new(stm, PieceType::Rook);

                self.remove_piece(rook, rook_from);
                self.add_piece(rook, rook_to);

                self.update_hash(rook, rook_from);
                self.update_hash(rook, rook_to);
            }
            _ if mv.is_promotion() => {
                let promotion = Piece::new(stm, mv.promotion_piece().unwrap());

                self.remove_piece(piece, to);
                self.add_piece(promotion, to);

                self.update_hash(piece, to);
                self.update_hash(promotion, to);
            }
            _ => (),
        }

        self.side_to_move = !self.side_to_move;

        self.state.castling.update(from, to);
        self.state.key ^= ZOBRIST.castling[self.state.castling];

        self.update_threats();
        self.update_king_threats();

        self.state.repetition = 0;

        let end = self.state.halfmove_clock.min(self.state.plies_from_null as u8);

        if end >= 4 {
            let mut idx = self.state_stack.len() as isize - 4;
            for i in (4..=end).step_by(2) {
                if idx < 0 {
                    break;
                }

                let stp = &self.state_stack[idx as usize];

                if stp.key == self.state.key {
                    self.state.repetition = if stp.repetition != 0 { -(i as i32) } else { i as i32 };
                    break;
                }

                idx -= 2;
            }
        }
    }

    pub fn undo_move(&mut self, mv: Move) {
        self.side_to_move = !self.side_to_move;

        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_on(to);
        let stm = self.side_to_move;

        self.add_piece(piece, from);
        self.remove_piece(piece, to);

        if let Some(piece) = self.state.captured {
            self.add_piece(piece, to);
        }

        match mv.kind() {
            MoveKind::EnPassant => {
                self.add_piece(Piece::new(!stm, PieceType::Pawn), to ^ 8);
            }
            MoveKind::Castling => {
                let (rook_from, root_to) = Self::get_castling_rook(to);
                self.add_piece(Piece::new(stm, PieceType::Rook), rook_from);
                self.remove_piece(Piece::new(stm, PieceType::Rook), root_to);
            }
            _ if mv.is_promotion() => {
                self.remove_piece(piece, from);
                self.add_piece(Piece::new(stm, PieceType::Pawn), from);
            }
            _ => (),
        }

        self.state = self.state_stack.pop().unwrap();
    }
}
