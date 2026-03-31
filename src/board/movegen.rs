use crate::{
    lookup::{
        between, bishop_attacks, king_attacks, knight_attacks, queen_attacks, ray_pass, relative_anti_diagonal,
        relative_diagonal, rook_attacks,
    },
    types::{Bitboard, CastlingKind, File, MoveKind, MoveList, PieceType, Square},
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
        !list.is_empty()
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
        let stm = self.side_to_move();
        self.collect_unpinned::<T, _>(
            list,
            !self.all_threats(),
            self.colored_pieces(stm, PieceType::King),
            king_attacks,
        );

        if self.checkers().is_multiple() {
            return;
        }

        let occupancies = self.occupancies();
        let target = if self.in_check() {
            between(self.king_square(self.side_to_move()), self.checkers().lsb()) | self.checkers().lsb().to_bb()
        } else {
            Bitboard::ALL
        };

        let pinned = self.pinned(self.side_to_move());

        self.collect_pawn_moves::<T>(list, target, pinned);

        let knights = self.colored_pieces(stm, PieceType::Knight);
        let bishops = self.colored_pieces(stm, PieceType::Bishop);
        let rooks = self.colored_pieces(stm, PieceType::Rook);
        let queens = self.colored_pieces(stm, PieceType::Queen);

        self.collect_unpinned::<T, _>(list, target, knights & !pinned, knight_attacks);
        self.collect_unpinned::<T, _>(list, target, bishops & !pinned, |sq| bishop_attacks(sq, occupancies));
        self.collect_unpinned::<T, _>(list, target, rooks & !pinned, |sq| rook_attacks(sq, occupancies));
        self.collect_unpinned::<T, _>(list, target, queens & !pinned, |sq| queen_attacks(sq, occupancies));

        self.collect_pinned::<T, _>(list, target, bishops & pinned, |sq| bishop_attacks(sq, occupancies));
        self.collect_pinned::<T, _>(list, target, rooks & pinned, |sq| rook_attacks(sq, occupancies));
        self.collect_pinned::<T, _>(list, target, queens & pinned, |sq| queen_attacks(sq, occupancies));

        if T::KIND == Kind::Quiet {
            self.collect_castling(list);
        }
    }

    fn collect_unpinned<T: MoveGenerator, F: Fn(Square) -> Bitboard>(
        &self, list: &mut MoveList, target: Bitboard, bb: Bitboard, attacks: F,
    ) {
        let stm = self.side_to_move();
        for from in bb {
            if T::KIND == Kind::Noisy {
                list.push_setwise(from, attacks(from) & target & self.colors(!stm), MoveKind::Capture);
            }
            if T::KIND == Kind::Quiet {
                list.push_setwise(from, attacks(from) & target & !self.occupancies(), MoveKind::Normal);
            }
        }
    }

    fn collect_pinned<T: MoveGenerator, F: Fn(Square) -> Bitboard>(
        &self, list: &mut MoveList, target: Bitboard, bb: Bitboard, attacks: F,
    ) {
        let king = self.king_square(self.side_to_move());
        let stm = self.side_to_move();
        for from in bb {
            let pin_mask = ray_pass(king, from);
            if T::KIND == Kind::Noisy {
                list.push_setwise(from, attacks(from) & target & self.colors(!stm) & pin_mask, MoveKind::Capture);
            }
            if T::KIND == Kind::Quiet {
                list.push_setwise(from, attacks(from) & target & !self.occupancies() & pin_mask, MoveKind::Normal);
            }
        }
    }

    fn collect_castling(&self, list: &mut MoveList) {
        let stm = self.side_to_move();
        self.collect_castling_kind(list, CastlingKind::KINGSIDE[stm]);
        self.collect_castling_kind(list, CastlingKind::QUEENSIDE[stm]);
    }

    fn collect_castling_kind(&self, list: &mut MoveList, kind: CastlingKind) {
        let stm = self.side_to_move();
        if self.castling().is_allowed(kind)
            && (self.castling_path[kind] & self.occupancies()).is_empty()
            && (self.castling_threat[kind] & self.all_threats()).is_empty()
            && !self.pinned(stm).contains(self.castling_rooks[kind])
        {
            let king = self.king_square(self.side_to_move());
            list.push(king, kind.landing_square(), MoveKind::Castling);
        }
    }

    fn collect_pawn_moves<T: MoveGenerator>(&self, list: &mut MoveList, target: Bitboard, pinned: Bitboard) {
        let pawns = self.colored_pieces(self.side_to_move(), PieceType::Pawn);
        let seventh_rank = Bitboard::SEVENTH_RANK[self.side_to_move()];

        self.collect_pawn_pushes::<T>(list, target, pinned, pawns, seventh_rank);

        if T::KIND == Kind::Noisy {
            self.collect_pawn_captures(list, target, pinned, pawns, seventh_rank);
        }
    }

    fn movable_pawns(pinned: Bitboard, pawns: Bitboard, pin_mask: Bitboard) -> Bitboard {
        (pawns & !pinned) | (pawns & pinned & pin_mask)
    }

    fn collect_pawn_pushes<T: MoveGenerator>(
        &self, list: &mut MoveList, target: Bitboard, pinned: Bitboard, pawns: Bitboard, seventh_rank: Bitboard,
    ) {
        let stm = self.side_to_move();
        let up = Square::UP[stm];
        let third_rank = Bitboard::THIRD_RANK[stm];
        let empty = !self.occupancies();
        let pawns = Self::movable_pawns(pinned, pawns, Bitboard::file(self.king_square(stm).file()));

        if T::KIND == Kind::Quiet {
            let non_promotions = pawns & !seventh_rank;
            let single_pushes = non_promotions.shift(up) & empty;
            let double_pushes = (single_pushes & third_rank).shift(up) & empty;

            list.push_pawns_setwise(up, single_pushes & target, MoveKind::Normal);
            list.push_pawns_setwise(up * 2, double_pushes & target, MoveKind::DoublePush);
        }

        let promotions = (pawns & seventh_rank).shift(up) & empty;
        if T::KIND == Kind::Noisy {
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionQ);
        }
        if T::KIND == Kind::Quiet {
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionR);
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionB);
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionN);
        }
    }

    fn collect_pawn_captures(
        &self, list: &mut MoveList, target: Bitboard, pinned: Bitboard, pawns: Bitboard, seventh_rank: Bitboard,
    ) {
        let stm = self.side_to_move();
        let up_right = Square::UP[stm] + Square::RIGHT;
        let up_left = Square::UP[stm] + Square::LEFT;
        let right_pin_mask = relative_diagonal(stm, self.king_square(stm));
        let left_pin_mask = relative_anti_diagonal(stm, self.king_square(stm));
        let right_pawns = Self::movable_pawns(pinned, pawns, right_pin_mask);
        let left_pawns = Self::movable_pawns(pinned, pawns, left_pin_mask);

        let right = (right_pawns & seventh_rank & !Bitboard::file(File::H)).shift(up_right) & self.colors(!stm);
        let left = (left_pawns & seventh_rank & !Bitboard::file(File::A)).shift(up_left) & self.colors(!stm);

        list.push_promotion_capture_setwise(up_right, right & target);
        list.push_promotion_capture_setwise(up_left, left & target);

        let right_captures =
            (right_pawns & !seventh_rank & !Bitboard::file(File::H)).shift(up_right) & self.colors(!stm);
        let left_captures = (left_pawns & !seventh_rank & !Bitboard::file(File::A)).shift(up_left) & self.colors(!stm);

        list.push_pawns_setwise(up_right, right_captures & target, MoveKind::Capture);
        list.push_pawns_setwise(up_left, left_captures & target, MoveKind::Capture);

        if self.en_passant() != Square::None {
            let ep = self.en_passant().to_bb();
            let right_attacker = right_pawns & !Bitboard::file(File::H) & ep.shift(-up_right);
            let left_attacker = left_pawns & !Bitboard::file(File::A) & ep.shift(-up_left);
            for pawn in right_attacker | left_attacker {
                list.push(pawn, self.en_passant(), MoveKind::EnPassant);
            }
        }
    }
}
