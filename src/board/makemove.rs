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

        let captured = self.piece_on(to);
        if captured != Piece::None {
            self.remove_piece(captured, to);
            self.state.captured = Some(captured);
        }

        self.remove_piece(piece, from);
        self.add_piece(piece, to);

        match mv.kind() {
            MoveKind::DoublePush => {
                self.state.en_passant = Square::new((from as u8 + to as u8) / 2);
                self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
            }
            MoveKind::EnPassant => {
                self.remove_piece(Piece::new(!stm, PieceType::Pawn), to ^ 8);
            }
            MoveKind::Castling => {
                let (rook_from, root_to) = Board::get_castling_rook(to);
                self.remove_piece(Piece::new(stm, PieceType::Rook), rook_from);
                self.add_piece(Piece::new(stm, PieceType::Rook), root_to);
            }
            _ if mv.is_promotion() => {
                self.remove_piece(Piece::new(stm, PieceType::Pawn), to);
                self.add_piece(Piece::new(stm, mv.promotion_piece().unwrap()), to);
            }
            _ => (),
        }

        self.side_to_move = !self.side_to_move;

        self.state.castling.update(from, to);
        self.state.key ^= ZOBRIST.castling[self.state.castling];

        self.update_threats();
        self.update_king_threats();
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
                let (rook_from, root_to) = Board::get_castling_rook(to);
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
