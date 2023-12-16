use super::{Board, InternalState};
use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, CastlingKind, Color, Move, MoveKind, MoveList, Piece, Square},
};

pub struct Generator<'a> {
    board: &'a Board,
    state: &'a InternalState,
    stm: Color,
    all: Bitboard,
    us: Bitboard,
    them: Bitboard,
    list: MoveList,
}

impl<'a> Generator<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            state: &board.state,
            stm: board.side_to_move,
            all: board.occupancies(),
            us: board.us(),
            them: board.them(),
            list: MoveList::new(),
        }
    }

    /// Generates pseudo legal moves for the current state of the board.
    pub fn generate(mut self) -> MoveList {
        let occupancies = self.all;

        self.collect_pawn_moves();

        self.collect_for(Piece::Knight, knight_attacks);
        self.collect_for(Piece::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for(Piece::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for(Piece::Queen, |square| queen_attacks(square, occupancies));

        self.collect_castling();
        self.collect_for(Piece::King, king_attacks);

        self.list
    }

    /// Adds move for the piece type using the specified move generator function.
    fn collect_for<T: Fn(Square) -> Bitboard>(&mut self, piece: Piece, gen: T) {
        for start in self.board.our(piece) {
            let targets = gen(start) & !self.us;

            self.add_many(start, targets & self.them, MoveKind::Capture);
            self.add_many(start, targets & !self.them, MoveKind::Quiet);
        }
    }

    fn collect_castling(&mut self) {
        match self.stm {
            Color::White => {
                self.collect_castling_kind(CastlingKind::WhiteShort);
                self.collect_castling_kind(CastlingKind::WhiteLong);
            }
            Color::Black => {
                self.collect_castling_kind(CastlingKind::BlackShort);
                self.collect_castling_kind(CastlingKind::BlackLong);
            }
        }
    }

    /// Adds the castling move to the move list if it's allowed.
    ///
    /// This method does not check if the king is in check after the castling,
    /// as this will be checked by the `make_move` method.
    fn collect_castling_kind(&mut self, kind: CastlingKind) {
        if (kind.path_mask() & self.all).is_empty() && self.state.castling.is_allowed(kind) {
            for square in kind.check_mask() {
                if self.board.is_under_attack(square) {
                    return;
                }
            }

            self.list.push(kind.castling_move());
        }
    }

    /// Adds all pawn moves to the move list.
    fn collect_pawn_moves(&mut self) {
        let bb = self.board.our(Piece::Pawn);

        let (starting_rank, promotion_rank) = match self.stm {
            Color::White => (Bitboard::RANK_2, Bitboard::RANK_7),
            Color::Black => (Bitboard::RANK_7, Bitboard::RANK_2),
        };

        self.collect_double_pushes(starting_rank & bb);
        self.collect_promotions(promotion_rank & bb);
        self.collect_regular_pawn_moves(!promotion_rank & bb);
        self.collect_en_passant_moves(bb);
    }

    /// Adds one square pawn pushes and regular captures to the move list.
    fn collect_regular_pawn_moves(&mut self, bb: Bitboard) {
        let offset = self.stm.offset();
        for start in bb {
            let captures = pawn_attacks(start, self.stm) & self.them;
            self.add_many(start, captures, MoveKind::Capture);

            let pawn_push = start.shift(offset);
            if !self.all.contains(pawn_push) {
                self.add(start, pawn_push, MoveKind::Quiet);
            }
        }
    }

    /// Adds promotions and capture promotions to the move list.
    fn collect_promotions(&mut self, bb: Bitboard) {
        let offset = self.stm.offset();
        for start in bb {
            let captures = pawn_attacks(start, self.stm) & self.them;
            for target in captures {
                self.add_promotion_captures(start, target);
            }

            let promotion = start.shift(offset);
            if !self.all.contains(promotion) {
                self.add_promotions(start, promotion);
            }
        }
    }

    // Adds double pawn pushes to the move list.
    fn collect_double_pushes(&mut self, bb: Bitboard) {
        let offset = self.stm.offset();
        for start in bb {
            let one_up = start.shift(offset);
            let two_up = one_up.shift(offset);

            if !self.all.contains(one_up) & !self.all.contains(two_up) {
                self.add(start, two_up, MoveKind::DoublePush);
            }
        }
    }

    /// Adds en passant captures to the move list.
    fn collect_en_passant_moves(&mut self, bb: Bitboard) {
        if self.state.en_passant != Square::None {
            let pawns = pawn_attacks(self.state.en_passant, !self.stm) & bb;
            for pawn in pawns {
                self.add(pawn, self.state.en_passant, MoveKind::EnPassant);
            }
        }
    }

    fn add(&mut self, start: Square, target: Square, move_kind: MoveKind) {
        self.list.push(Move::new(start, target, move_kind));
    }

    /// Adds all possible moves from the given starting square to the squares of the `targets` bitboard.
    fn add_many(&mut self, start: Square, targets: Bitboard, move_kind: MoveKind) {
        for target in targets {
            self.add(start, target, move_kind);
        }
    }

    /// Adds all possible promotion moves to the move list.
    fn add_promotions(&mut self, start: Square, target: Square) {
        self.add(start, target, MoveKind::PromotionQ);
        self.add(start, target, MoveKind::PromotionR);
        self.add(start, target, MoveKind::PromotionB);
        self.add(start, target, MoveKind::PromotionN);
    }

    /// Adds all possible promotion captures to the move list.
    fn add_promotion_captures(&mut self, start: Square, target: Square) {
        self.add(start, target, MoveKind::PromotionCaptureQ);
        self.add(start, target, MoveKind::PromotionCaptureR);
        self.add(start, target, MoveKind::PromotionCaptureB);
        self.add(start, target, MoveKind::PromotionCaptureN);
    }
}
