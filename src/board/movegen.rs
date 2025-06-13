use crate::{
    lookup::{between, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, CastlingKind, Color, File, MoveKind, MoveList, PieceType, Rank, Square},
};

const QUIET: u8 = 0;
const NOISY: u8 = 1;

impl super::Board {
    pub fn has_legal_moves(&self) -> bool {
        let mut list = MoveList::new();
        self.append_all_moves(&mut list);
        list.iter().any(|entry| self.is_legal(entry.mv))
    }

    /// Generates all possible pseudo legal moves for the current position.
    pub fn generate_all_moves(&self) -> MoveList {
        let mut list = MoveList::new();
        self.append_all_moves(&mut list);
        list
    }

    pub fn append_all_moves(&self, list: &mut MoveList) {
        self.generate_moves::<QUIET>(list);
        self.generate_moves::<NOISY>(list);
    }

    pub fn append_quiet_moves(&self, list: &mut MoveList) {
        self.generate_moves::<QUIET>(list);
    }

    /// Generates only pseudo legal capture moves for the current position.
    pub fn append_noisy_moves(&self, list: &mut MoveList) {
        self.generate_moves::<NOISY>(list);
    }

    /// Generates pseudo legal moves for the current position.
    fn generate_moves<const TYPE: u8>(&self, list: &mut MoveList) {
        let king = self.king_square(self.side_to_move);
        let checkers = self.checkers();

        self.collect_king_moves::<TYPE>(list, king);

        if checkers.multiple() {
            return;
        }

        let occupancies = self.occupancies();
        let mut target = match TYPE {
            NOISY => self.them(),
            QUIET => !self.occupancies(),
            _ => unreachable!(),
        };

        if self.in_check() {
            target &= between(king, checkers.lsb()) | checkers.lsb().to_bb();
        }

        self.collect_pawn_moves::<TYPE>(list);

        self.collect_for::<TYPE, _>(list, target, PieceType::Knight, knight_attacks);
        self.collect_for::<TYPE, _>(list, target, PieceType::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for::<TYPE, _>(list, target, PieceType::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for::<TYPE, _>(list, target, PieceType::Queen, |square| queen_attacks(square, occupancies));

        if TYPE == QUIET && checkers.is_empty() {
            self.collect_castling(list);
        }
    }

    /// Adds move for the piece type using the specified move generator function.
    fn collect_for<const TYPE: u8, T>(&self, list: &mut MoveList, target: Bitboard, piece: PieceType, gen: T)
    where
        T: Fn(Square) -> Bitboard,
    {
        for from in self.our(piece) {
            if TYPE == NOISY {
                for to in gen(from) & target {
                    list.push(from, to, MoveKind::Capture);
                }
            }

            if TYPE == QUIET {
                for to in gen(from) & target {
                    list.push(from, to, MoveKind::Normal);
                }
            }
        }
    }

    fn collect_king_moves<const TYPE: u8>(&self, list: &mut MoveList, king: Square) {
        if TYPE == NOISY {
            for to in king_attacks(king) & self.them() {
                list.push(king, to, MoveKind::Capture);
            }
        }

        if TYPE == QUIET {
            for to in king_attacks(king) & !self.occupancies() {
                list.push(king, to, MoveKind::Normal);
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

            list.push_move(KIND::CASTLING_MOVE);
        }
    }

    /// Adds all pawn moves to the move list.
    fn collect_pawn_moves<const TYPE: u8>(&self, list: &mut MoveList) {
        let pawns = self.our(PieceType::Pawn);
        let seventh_rank = match self.side_to_move {
            Color::White => Bitboard::rank(Rank::R7),
            Color::Black => Bitboard::rank(Rank::R2),
        };

        self.collect_pawn_pushes::<TYPE>(list, pawns, seventh_rank);

        if TYPE == NOISY {
            self.collect_pawn_captures(list, pawns, seventh_rank);
            self.collect_en_passant_moves(list, pawns);
        }
    }

    /// Adds single, double and promotion pawn pushes to the move list.
    fn collect_pawn_pushes<const TYPE: u8>(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let (up, third_rank) = match self.side_to_move {
            Color::White => (8, Bitboard::rank(Rank::R3)),
            Color::Black => (-8, Bitboard::rank(Rank::R6)),
        };

        let empty = !self.occupancies();

        if TYPE == QUIET {
            let non_promotions = pawns & !seventh_rank;
            let single_pushes = non_promotions.shift(up) & empty;
            let double_pushes = (single_pushes & third_rank).shift(up) & empty;

            for to in single_pushes {
                list.push(to.shift(-up), to, MoveKind::Normal);
            }

            for to in double_pushes {
                list.push(to.shift(-up * 2), to, MoveKind::DoublePush);
            }
        }

        let promotions = (pawns & seventh_rank).shift(up) & empty;
        for to in promotions {
            let from = to.shift(-up);

            if TYPE == NOISY {
                list.push(from, to, MoveKind::PromotionQ);
            }

            if TYPE == QUIET {
                list.push(from, to, MoveKind::PromotionR);
                list.push(from, to, MoveKind::PromotionB);
                list.push(from, to, MoveKind::PromotionN);
            }
        }
    }

    /// Adds regular pawn captures and promotion captures to the move list.
    fn collect_pawn_captures(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        fn add_promotions(list: &mut MoveList, from: Square, to: Square) {
            list.push(from, to, MoveKind::PromotionCaptureQ);
            list.push(from, to, MoveKind::PromotionCaptureR);
            list.push(from, to, MoveKind::PromotionCaptureB);
            list.push(from, to, MoveKind::PromotionCaptureN);
        }

        let (up_right, up_left) = match self.side_to_move {
            Color::White => (9, 7),
            Color::Black => (-7, -9),
        };

        let promotions = pawns & seventh_rank;
        let right = (promotions & !Bitboard::file(File::H)).shift(up_right) & self.them();
        let left = (promotions & !Bitboard::file(File::A)).shift(up_left) & self.them();

        for to in right {
            add_promotions(list, to.shift(-up_right), to);
        }
        for to in left {
            add_promotions(list, to.shift(-up_left), to);
        }

        let non_promotions = pawns & !seventh_rank;
        let right_captures = (non_promotions & !Bitboard::file(File::H)).shift(up_right) & self.them();
        let left_captures = (non_promotions & !Bitboard::file(File::A)).shift(up_left) & self.them();

        for to in right_captures {
            list.push(to.shift(-up_right), to, MoveKind::Capture);
        }
        for to in left_captures {
            list.push(to.shift(-up_left), to, MoveKind::Capture);
        }
    }

    fn collect_en_passant_moves(&self, list: &mut MoveList, pawns: Bitboard) {
        if self.state.en_passant != Square::None {
            let pawns = pawns & pawn_attacks(self.state.en_passant, !self.side_to_move);
            for pawn in pawns {
                list.push(pawn, self.state.en_passant, MoveKind::EnPassant);
            }
        }
    }
}
