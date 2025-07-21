use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, CastlingKind, Color, File, MoveKind, MoveList, PieceType, Rank, Square},
};

#[derive(Eq, PartialEq)]
enum Kind {
    Quiet,
    Noisy,
}

trait MoveGenerator {
    const KIND: Kind;
}

struct Quiet;
impl MoveGenerator for Quiet {
    const KIND: Kind = Kind::Quiet;
}

struct Noisy;
impl MoveGenerator for Noisy {
    const KIND: Kind = Kind::Noisy;
}

impl super::Board {
    pub fn has_legal_moves(&self) -> bool {
        let mut list = MoveList::new();
        self.append_all_moves(&mut list);
        list.iter().any(|entry| self.is_legal(entry.mv))
    }

    pub fn generate_all_moves(&self) -> MoveList {
        let mut list = MoveList::new();
        self.append_all_moves(&mut list);
        list
    }

    pub fn append_all_moves(&self, list: &mut MoveList) {
        self.append_noisy_moves(list);
        self.append_quiet_moves(list);
    }

    pub fn append_quiet_moves(&self, list: &mut MoveList) {
        self.generate_moves::<Quiet>(list);
    }

    pub fn append_noisy_moves(&self, list: &mut MoveList) {
        self.generate_moves::<Noisy>(list);
    }

    fn generate_moves<T: MoveGenerator>(&self, list: &mut MoveList) {
        self.collect_for::<T, _>(list, Bitboard::ALL, PieceType::King, king_attacks);

        if self.checkers().multiple() {
            return;
        }

        let occupancies = self.occupancies();
        let target = Bitboard::ALL;

        self.collect_pawn_moves::<T>(list);

        self.collect_for::<T, _>(list, target, PieceType::Knight, knight_attacks);
        self.collect_for::<T, _>(list, target, PieceType::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for::<T, _>(list, target, PieceType::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for::<T, _>(list, target, PieceType::Queen, |square| queen_attacks(square, occupancies));

        if T::KIND == Kind::Quiet {
            self.collect_castling(list);
        }
    }

    fn collect_for<T: MoveGenerator, F: Fn(Square) -> Bitboard>(
        &self, list: &mut MoveList, target: Bitboard, piece: PieceType, attacks: F,
    ) {
        for from in self.our(piece) {
            if T::KIND == Kind::Noisy {
                for to in attacks(from) & target & self.them() {
                    list.push(from, to, MoveKind::Capture);
                }
            }

            if T::KIND == Kind::Quiet {
                for to in attacks(from) & target & !self.occupancies() {
                    list.push(from, to, MoveKind::Normal);
                }
            }
        }
    }

    fn collect_castling(&self, list: &mut MoveList) {
        match self.side_to_move {
            Color::White => {
                self.collect_castling_kind(list, CastlingKind::WhiteKinside);
                self.collect_castling_kind(list, CastlingKind::WhiteQueenside);
            }
            Color::Black => {
                self.collect_castling_kind(list, CastlingKind::BlackKingside);
                self.collect_castling_kind(list, CastlingKind::BlackQueenside);
            }
        }
    }

    fn collect_castling_kind(&self, list: &mut MoveList, kind: CastlingKind) {
        if self.castling().is_allowed(kind)
            && (self.castling_path[kind] & self.occupancies()).is_empty()
            && (self.castling_threat[kind] & self.threats()).is_empty()
        {
            let king = self.king_square(self.side_to_move);
            list.push(king, kind.landing_square(), MoveKind::Castling);
        }
    }

    fn collect_pawn_moves<T: MoveGenerator>(&self, list: &mut MoveList) {
        let pawns = self.our(PieceType::Pawn);
        let seventh_rank = match self.side_to_move {
            Color::White => Bitboard::rank(Rank::R7),
            Color::Black => Bitboard::rank(Rank::R2),
        };

        self.collect_pawn_pushes::<T>(list, pawns, seventh_rank);

        if T::KIND == Kind::Noisy {
            self.collect_pawn_captures(list, pawns, seventh_rank);
            self.collect_en_passant_moves(list, pawns);
        }
    }

    fn collect_pawn_pushes<T: MoveGenerator>(&self, list: &mut MoveList, pawns: Bitboard, seventh_rank: Bitboard) {
        let (up, third_rank) = match self.side_to_move {
            Color::White => (8, Bitboard::rank(Rank::R3)),
            Color::Black => (-8, Bitboard::rank(Rank::R6)),
        };

        let empty = !self.occupancies();

        if T::KIND == Kind::Quiet {
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

            if T::KIND == Kind::Noisy {
                list.push(from, to, MoveKind::PromotionQ);
            }

            if T::KIND == Kind::Quiet {
                list.push(from, to, MoveKind::PromotionR);
                list.push(from, to, MoveKind::PromotionB);
                list.push(from, to, MoveKind::PromotionN);
            }
        }
    }

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
