use super::{zobrist::ZOBRIST, Board};
use crate::types::{Move, MoveKind, Piece, PieceType, Square};

impl Board {
    pub fn make_null_move(&mut self) {
        self.side_to_move = !self.side_to_move;
        self.move_stack.push(Move::NULL);
        self.state_stack.push(self.state);

        self.state.hash_key ^= ZOBRIST.side;
        self.state.hash_key ^= ZOBRIST.castling[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash_key ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }
    }

    pub fn undo_null_move(&mut self) {
        self.side_to_move = !self.side_to_move;
        self.state = self.state_stack.pop().unwrap();
        self.move_stack.pop();
    }

    pub fn make_move<const NNUE: bool, const IN_PLACE: bool>(&mut self, mv: Move) -> bool {
        let start = mv.start();
        let target = mv.target();
        let piece = self.piece_on(start);
        let pt = piece.piece_type();
        let stm = self.side_to_move;

        self.move_stack.push(mv);
        self.state_stack.push(self.state);

        if NNUE && !IN_PLACE {
            self.nnue.push();
        }

        self.state.hash_key ^= ZOBRIST.side;
        self.state.hash_key ^= ZOBRIST.castling[self.state.castling];

        if self.state.en_passant != Square::None {
            self.state.hash_key ^= ZOBRIST.en_passant[self.state.en_passant];
            self.state.en_passant = Square::None;
        }

        self.state.captured = None;

        if mv.is_capture() || pt == PieceType::Pawn {
            self.state.halfmove_clock = 0;
        } else {
            self.state.halfmove_clock += 1;
        }

        let captured = self.piece_on(target);
        if captured != Piece::None {
            self.remove_piece::<NNUE>(captured, target);
            self.state.captured = Some(captured);
        }

        self.remove_piece::<NNUE>(piece, start);
        self.add_piece::<NNUE>(piece, target);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = Square::new((start as u8 + target as u8) / 2);
                self.state.hash_key ^= ZOBRIST.en_passant[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece::<NNUE>(Piece::new(!stm, PieceType::Pawn), target ^ 8);
            }
            MoveKind::Castling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.remove_piece::<NNUE>(Piece::new(stm, PieceType::Rook), rook_start);
                self.add_piece::<NNUE>(Piece::new(stm, PieceType::Rook), rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece::<NNUE>(Piece::new(stm, PieceType::Pawn), target);
                self.add_piece::<NNUE>(Piece::new(stm, mv.promotion_piece().unwrap()), target);
            }
            _ => (),
        }

        self.state.castling.update(start, target);
        self.state.hash_key ^= ZOBRIST.castling[self.state.castling];
        self.side_to_move = !self.side_to_move;

        if NNUE {
            self.nnue.commit();
        }

        let king = self.their(PieceType::King).lsb();
        !self.is_square_attacked_by(king, self.side_to_move)
    }

    pub fn undo_move<const NNUE: bool>(&mut self) {
        if NNUE {
            self.nnue.pop();
        }

        self.side_to_move = !self.side_to_move;

        let mv = self.move_stack.pop().unwrap();
        let start = mv.start();
        let target = mv.target();
        let piece = self.piece_on(target);
        let stm = self.side_to_move;

        self.add_piece::<false>(piece, start);
        self.remove_piece::<false>(piece, target);

        if let Some(piece) = self.state.captured {
            self.add_piece::<false>(piece, target);
        }

        match mv.kind() {
            MoveKind::EnPassant => {
                self.add_piece::<false>(Piece::new(!stm, PieceType::Pawn), target ^ 8);
            }
            MoveKind::Castling => {
                let (rook_start, rook_target) = get_rook_move(target);
                self.add_piece::<false>(Piece::new(stm, PieceType::Rook), rook_start);
                self.remove_piece::<false>(Piece::new(stm, PieceType::Rook), rook_target);
            }
            _ if mv.is_promotion() => {
                self.remove_piece::<false>(piece, start);
                self.add_piece::<false>(Piece::new(stm, PieceType::Pawn), start);
            }
            _ => (),
        }

        self.state = self.state_stack.pop().unwrap();
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
