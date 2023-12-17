use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, CastlingKind, Color, MoveKind, MoveList, Piece, Rank, Square},
};

impl super::Board {
    /// Generates all possible pseudo legal moves for the current position.
    pub fn generate_moves(&self) -> MoveList {
        let occupancies = self.occupancies();

        let mut list = MoveList::new();

        self.collect_pawn_moves(&mut list);

        self.collect_for(&mut list, Piece::Knight, knight_attacks);
        self.collect_for(&mut list, Piece::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for(&mut list, Piece::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for(&mut list, Piece::Queen, |square| queen_attacks(square, occupancies));

        self.collect_castling(&mut list);
        self.collect_for(&mut list, Piece::King, king_attacks);

        list
    }

    /// Adds move for the piece type using the specified move generator function.
    fn collect_for<T: Fn(Square) -> Bitboard>(&self, list: &mut MoveList, piece: Piece, gen: T) {
        for start in self.our(piece) {
            let targets = gen(start) & !self.us();

            list.add_many(start, targets & self.them(), MoveKind::Capture);
            list.add_many(start, targets & !self.them(), MoveKind::Quiet);
        }
    }

    fn collect_castling(&self, list: &mut MoveList) {
        use crate::types::{BlackKingSide, BlackQueenSide, WhiteKingSide, WhiteQueenSide};

        match self.side_to_move {
            Color::White => {
                self.collect_castling_kind::<WhiteKingSide>(list);
                self.collect_castling_kind::<WhiteQueenSide>(list);
            }
            Color::Black => {
                self.collect_castling_kind::<BlackKingSide>(list);
                self.collect_castling_kind::<BlackQueenSide>(list);
            }
        }
    }

    /// Adds the castling move to the move list if it's allowed.
    ///
    /// This method does not check if the king is in check after the castling,
    /// as this will be checked by the `make_move` method.
    fn collect_castling_kind<KIND: CastlingKind>(&self, list: &mut MoveList) {
        if (KIND::PATH_MASK & self.occupancies()).is_empty() && self.state.castling.is_allowed::<KIND>() {
            for square in KIND::CHECK_SQUARES {
                if self.is_under_attack(square) {
                    return;
                }
            }

            list.push(KIND::CASTLING_MOVE);
        }
    }

    /// Adds all pawn moves to the move list.
    fn collect_pawn_moves(&self, list: &mut MoveList) {
        let pawns = self.our(Piece::Pawn);
        let seventh_rank = match self.side_to_move {
            Color::White => Bitboard::rank(Rank::R7),
            Color::Black => Bitboard::rank(Rank::R2),
        };

        self.collect_pawn_pushes(list, pawns, seventh_rank);
        self.collect_pawn_captures(list, pawns, seventh_rank);
        self.collect_en_passant_moves(list, pawns);
    }

    /// Adds single, double and promotion pawn pushes to the move list.
    fn collect_pawn_pushes(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let (up, third_rank) = match self.side_to_move {
            Color::White => (8, Bitboard::rank(Rank::R3)),
            Color::Black => (-8, Bitboard::rank(Rank::R6)),
        };

        let empty = !self.occupancies();

        let non_promotions = pawns & !seventh_rank;
        let single_pushes = non_promotions.shift(up) & empty;
        let double_pushes = (single_pushes & third_rank).shift(up) & empty;

        for target in single_pushes {
            list.add(target.shift(-up), target, MoveKind::Quiet);
        }

        for target in double_pushes {
            list.add(target.shift(-up * 2), target, MoveKind::DoublePush);
        }

        let promotions = (pawns & seventh_rank).shift(up) & empty;
        for target in promotions {
            let start = target.shift(-up);
            list.add(start, target, MoveKind::PromotionQ);
            list.add(start, target, MoveKind::PromotionR);
            list.add(start, target, MoveKind::PromotionB);
            list.add(start, target, MoveKind::PromotionN);
        }
    }

    /// Adds regular pawn captures and promotion captures to the move list.
    fn collect_pawn_captures(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let promotions = pawns & seventh_rank;
        for start in promotions {
            let captures = self.them() & pawn_attacks(start, self.side_to_move);
            for target in captures {
                list.add(start, target, MoveKind::PromotionCaptureQ);
                list.add(start, target, MoveKind::PromotionCaptureR);
                list.add(start, target, MoveKind::PromotionCaptureB);
                list.add(start, target, MoveKind::PromotionCaptureN);
            }
        }

        let non_promotions = pawns & !seventh_rank;
        for start in non_promotions {
            let targets = self.them() & pawn_attacks(start, self.side_to_move);
            list.add_many(start, targets, MoveKind::Capture);
        }
    }

    fn collect_en_passant_moves(&self, list: &mut MoveList, pawns: Bitboard) {
        if self.state.en_passant != Square::None {
            let pawns = pawns & pawn_attacks(self.state.en_passant, !self.side_to_move);
            for pawn in pawns {
                list.add(pawn, self.state.en_passant, MoveKind::EnPassant);
            }
        }
    }
}
