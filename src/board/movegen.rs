use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{ArrayVec, Bitboard, CastlingKind, Color, Move, MoveKind, PieceType, Rank, Square, MAX_MOVES},
};

macro_rules! push {
    ($list:ident, $from:expr, $to:expr, $kind:expr) => {
        $list.push(Move::new($from, $to, $kind));
    };
}

type MoveList = ArrayVec<Move, MAX_MOVES>;

impl super::Board {
    /// Generates all possible pseudo legal moves for the current position.
    pub fn generate_all_moves(&self) -> MoveList {
        let mut list = MoveList::new();
        self.append_all_moves(&mut list);
        list
    }

    pub fn append_all_moves(&self, list: &mut MoveList) {
        self.generate_moves::<false>(list)
    }

    /// Generates only pseudo legal capture moves for the current position.
    pub fn append_noisy_moves(&self, list: &mut MoveList) {
        self.generate_moves::<true>(list)
    }

    /// Generates pseudo legal moves for the current position.
    fn generate_moves<const NOISY: bool>(&self, list: &mut MoveList) {
        let occupancies = self.occupancies();

        self.collect_pawn_moves::<NOISY>(list);

        self.collect_for::<NOISY, _>(list, PieceType::Knight, knight_attacks);
        self.collect_for::<NOISY, _>(list, PieceType::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for::<NOISY, _>(list, PieceType::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for::<NOISY, _>(list, PieceType::Queen, |square| queen_attacks(square, occupancies));
        self.collect_for::<NOISY, _>(list, PieceType::King, king_attacks);

        if !NOISY {
            self.collect_castling(list);
        }
    }

    /// Adds move for the piece type using the specified move generator function.
    fn collect_for<const NOISY: bool, T>(&self, list: &mut MoveList, piece: PieceType, gen: T)
    where
        T: Fn(Square) -> Bitboard,
    {
        for from in self.our(piece) {
            let targets = gen(from) & !self.us();

            for to in targets & self.them() {
                push!(list, from, to, MoveKind::Capture);
            }

            if !NOISY {
                for to in targets & !self.them() {
                    push!(list, from, to, MoveKind::Normal);
                }
            }
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
                if self.is_threatened(square) {
                    return;
                }
            }

            list.push(KIND::CASTLING_MOVE);
        }
    }

    /// Adds all pawn moves to the move list.
    fn collect_pawn_moves<const NOISY: bool>(&self, list: &mut MoveList) {
        let pawns = self.our(PieceType::Pawn);
        let seventh_rank = match self.side_to_move {
            Color::White => Bitboard::rank(Rank::R7),
            Color::Black => Bitboard::rank(Rank::R2),
        };

        self.collect_pawn_pushes::<NOISY>(list, pawns, seventh_rank);
        self.collect_pawn_captures::<NOISY>(list, pawns, seventh_rank);

        self.collect_en_passant_moves(list, pawns);
    }

    /// Adds single, double and promotion pawn pushes to the move list.
    fn collect_pawn_pushes<const NOISY: bool>(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let (up, third_rank) = match self.side_to_move {
            Color::White => (8, Bitboard::rank(Rank::R3)),
            Color::Black => (-8, Bitboard::rank(Rank::R6)),
        };

        let empty = !self.occupancies();

        if !NOISY {
            let non_promotions = pawns & !seventh_rank;
            let single_pushes = non_promotions.shift(up) & empty;
            let double_pushes = (single_pushes & third_rank).shift(up) & empty;

            for to in single_pushes {
                push!(list, to.shift(-up), to, MoveKind::Normal);
            }

            for to in double_pushes {
                push!(list, to.shift(-up * 2), to, MoveKind::DoublePush);
            }
        }

        let promotions = (pawns & seventh_rank).shift(up) & empty;
        for to in promotions {
            let from = to.shift(-up);
            push!(list, from, to, MoveKind::PromotionQ);

            if !NOISY {
                push!(list, from, to, MoveKind::PromotionR);
                push!(list, from, to, MoveKind::PromotionB);
                push!(list, from, to, MoveKind::PromotionN);
            }
        }
    }

    /// Adds regular pawn captures and promotion captures to the move list.
    fn collect_pawn_captures<const NOISY: bool>(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let promotions = pawns & seventh_rank;
        for from in promotions {
            let captures = self.them() & pawn_attacks(from, self.side_to_move);
            for to in captures {
                push!(list, from, to, MoveKind::PromotionCaptureQ);

                if !NOISY {
                    push!(list, from, to, MoveKind::PromotionCaptureR);
                    push!(list, from, to, MoveKind::PromotionCaptureB);
                    push!(list, from, to, MoveKind::PromotionCaptureN);
                }
            }
        }

        let non_promotions = pawns & !seventh_rank;
        for from in non_promotions {
            let targets = self.them() & pawn_attacks(from, self.side_to_move);
            for to in targets {
                push!(list, from, to, MoveKind::Capture);
            }
        }
    }

    fn collect_en_passant_moves(&self, list: &mut MoveList, pawns: Bitboard) {
        if self.state.en_passant != Square::None {
            let pawns = pawns & pawn_attacks(self.state.en_passant, !self.side_to_move);
            for pawn in pawns {
                push!(list, pawn, self.state.en_passant, MoveKind::EnPassant);
            }
        }
    }
}
