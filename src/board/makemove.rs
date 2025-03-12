use super::{zobrist::ZOBRIST, Board};
use crate::types::{Move, MoveKind, Piece, PieceType, Square};

impl Board {
    pub fn make_null_move(&mut self) {
        self.side_to_move = !self.side_to_move;
        self.state_stack.push(self.state);

        self.state.key ^= ZOBRIST.side;
        self.state.key ^= ZOBRIST.castling[self.state.castling];

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

    pub fn make_move<const NNUE: bool, const IN_PLACE: bool>(&mut self, mv: Move) {
        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_on(from);
        let pt = piece.piece_type();
        let stm = self.side_to_move;

        self.state_stack.push(self.state);

        if NNUE && !IN_PLACE {
            self.nnue.push();
        }

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

        let captured = self.piece_on(to);
        if captured != Piece::None {
            self.remove_piece::<NNUE>(captured, to);
            self.state.captured = Some(captured);
        }

        self.remove_piece::<NNUE>(piece, from);
        self.add_piece::<NNUE>(piece, to);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = Square::new((from as u8 + to as u8) / 2);
                self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece::<NNUE>(Piece::new(!stm, PieceType::Pawn), to ^ 8);
            }
            MoveKind::Castling => {
                let (rook_from, root_to) = get_rook_move(to);
                self.remove_piece::<NNUE>(Piece::new(stm, PieceType::Rook), rook_from);
                self.add_piece::<NNUE>(Piece::new(stm, PieceType::Rook), root_to);
            }
            _ if mv.is_promotion() => {
                self.remove_piece::<NNUE>(Piece::new(stm, PieceType::Pawn), to);
                self.add_piece::<NNUE>(Piece::new(stm, mv.promotion_piece().unwrap()), to);
            }
            _ => (),
        }

        self.side_to_move = !self.side_to_move;

        self.state.castling.update(from, to);
        self.state.key ^= ZOBRIST.castling[self.state.castling];

        self.update_threats();
        self.update_king_threats();

        if NNUE {
            self.nnue.commit();
        }
    }

    pub fn undo_move<const NNUE: bool>(&mut self, mv: Move) {
        if NNUE {
            self.nnue.pop();
        }

        self.side_to_move = !self.side_to_move;

        let from = mv.from();
        let to = mv.to();
        let piece = self.piece_on(to);
        let stm = self.side_to_move;

        self.add_piece::<false>(piece, from);
        self.remove_piece::<false>(piece, to);

        if let Some(piece) = self.state.captured {
            self.add_piece::<false>(piece, to);
        }

        match mv.kind() {
            MoveKind::EnPassant => {
                self.add_piece::<false>(Piece::new(!stm, PieceType::Pawn), to ^ 8);
            }
            MoveKind::Castling => {
                let (rook_from, root_to) = get_rook_move(to);
                self.add_piece::<false>(Piece::new(stm, PieceType::Rook), rook_from);
                self.remove_piece::<false>(Piece::new(stm, PieceType::Rook), root_to);
            }
            _ if mv.is_promotion() => {
                self.remove_piece::<false>(piece, from);
                self.add_piece::<false>(Piece::new(stm, PieceType::Pawn), from);
            }
            _ => (),
        }

        self.state = self.state_stack.pop().unwrap();
    }
}

const fn get_rook_move(king_to: Square) -> (Square, Square) {
    match king_to {
        Square::G1 => (Square::H1, Square::F1),
        Square::C1 => (Square::A1, Square::D1),
        Square::G8 => (Square::H8, Square::F8),
        Square::C8 => (Square::A8, Square::D8),
        _ => unreachable!(),
    }
}
