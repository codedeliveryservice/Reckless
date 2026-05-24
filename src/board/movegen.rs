use crate::{
    lookup::{
        between, bishop_attacks, king_attacks, knight_attacks, queen_attacks, ray_pass, relative_diagonal, rook_attacks,
    },
    types::{Bitboard, CastlingKind, File, MoveKind, MoveList, PieceType, Square},
};

#[derive(Copy, Clone, Eq, PartialEq)]
enum MovegenKind {
    Quiet,
    Noisy,
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
        self.generate_moves(list, MovegenKind::Quiet);
    }

    pub fn append_noisy_moves(&self, list: &mut MoveList) {
        self.generate_moves(list, MovegenKind::Noisy);
    }

    fn generate_moves(&self, list: &mut MoveList, mgkind: MovegenKind) {
        let stm = self.side_to_move();
        let occupancies = self.occupancies();
        let kind_target = if mgkind == MovegenKind::Quiet { !occupancies } else { self.colors(!stm) };
        let move_kind = if mgkind == MovegenKind::Quiet { MoveKind::Normal } else { MoveKind::Capture };

        let king_sq = self.king_square(stm);
        list.push_setwise(king_sq, king_attacks(king_sq) & !self.all_threats() & kind_target, move_kind);

        if self.checkers().is_multiple() {
            return;
        }

        let mut target =
            if self.in_check() { between(king_sq, self.checkers().lsb()) | self.checkers() } else { Bitboard::ALL };
        let pinned = self.pinned(stm);

        self.collect_pawn_moves(list, target, pinned, mgkind); //broken noisy/quiet boundary

        target &= kind_target;

        for knight in self.colored_pieces(stm, PieceType::Knight) & !pinned {
            list.push_setwise(knight, knight_attacks(knight) & target, move_kind);
        }

        let bishops = self.colored_pieces(stm, PieceType::Bishop);
        let rooks = self.colored_pieces(stm, PieceType::Rook);
        let queens = self.colored_pieces(stm, PieceType::Queen);

        self.collect::<_>(list, target, bishops, move_kind, pinned, |sq| bishop_attacks(sq, occupancies));
        self.collect::<_>(list, target, rooks, move_kind, pinned, |sq| rook_attacks(sq, occupancies));
        self.collect::<_>(list, target, queens, move_kind, pinned, |sq| queen_attacks(sq, occupancies));

        if mgkind == MovegenKind::Quiet {
            self.collect_castling(list);
        }
    }

    fn collect<F: Fn(Square) -> Bitboard>(
        &self, list: &mut MoveList, target: Bitboard, pieces: Bitboard, move_kind: MoveKind, pinned: Bitboard,
        attacks: F,
    ) {
        for from in pieces & !pinned {
            list.push_setwise(from, attacks(from) & target, move_kind);
        }

        let king_sq = self.king_square(self.side_to_move());
        for from in pieces & pinned {
            let pin_mask = ray_pass(king_sq, from);
            list.push_setwise(from, attacks(from) & target & pin_mask, move_kind);
        }
    }

    fn collect_castling(&self, list: &mut MoveList) {
        let stm = self.side_to_move();
        for kind in [CastlingKind::KINDS[stm][0], CastlingKind::KINDS[stm][1]] {
            if self.castling().is_allowed(kind)
                && (self.castling_path[kind] & self.occupancies()).is_empty()
                && (self.castling_threat[kind] & self.all_threats()).is_empty()
                && !self.pinned(stm).contains(self.castling_rooks[kind])
            {
                list.push(self.king_square(stm), kind.landing_square(), MoveKind::Castling);
            }
        }
    }

    fn collect_pawn_captures(
        &self, list: &mut MoveList, pawns: Bitboard, dir: i8, target: Bitboard, seventh_rank: Bitboard,
    ) {
        let promos = (pawns & seventh_rank).shift(dir);
        list.push_promotion_capture_setwise(dir, promos & target);
        let captures = (pawns & !seventh_rank).shift(dir);
        list.push_pawns_setwise(dir, captures & target, MoveKind::Capture);

        let ep = self.en_passant();
        if ep != Square::None && pawns.contains(ep.shift(-dir)) {
            list.push(ep.shift(-dir), self.en_passant(), MoveKind::EnPassant);
        }
    }

    fn collect_pawn_moves(&self, list: &mut MoveList, target: Bitboard, pinned: Bitboard, mgkind: MovegenKind) {
        let stm = self.side_to_move();
        let up = Square::UP[stm];
        let pawns = self.colored_pieces(stm, PieceType::Pawn);
        let seventh_rank = Bitboard::SEVENTH_RANK[stm];
        let third_rank = Bitboard::THIRD_RANK[stm];
        let empty = !self.occupancies();
        let king_sq = self.king_square(stm);

        let pushable_pawns = pawns & (!pinned | Bitboard::file(king_sq.file()));
        let promotions = (pushable_pawns & seventh_rank).shift(up) & empty;

        if mgkind == MovegenKind::Quiet {
            let non_promotions = pushable_pawns & !seventh_rank;
            let single_pushes = non_promotions.shift(up) & empty;
            let double_pushes = (single_pushes & third_rank).shift(up) & empty;

            list.push_pawns_setwise(up, single_pushes & target, MoveKind::Normal);
            list.push_pawns_setwise(up * 2, double_pushes & target, MoveKind::DoublePush);
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionR);
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionB);
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionN);
        }

        if mgkind == MovegenKind::Noisy {
            list.push_pawns_setwise(up, promotions & target, MoveKind::PromotionQ);

            let target = target & self.colors(!stm);

            let dirs = [up + Square::RIGHT, up + Square::LEFT];
            let pin_masks = [relative_diagonal(stm, king_sq), relative_diagonal(!stm, king_sq)];
            let shift_masks = [!Bitboard::file(File::H), !Bitboard::file(File::A)];

            for i in 0..2 {
                let the_pawns = pawns & (!pinned | pin_masks[i]) & shift_masks[i];
                self.collect_pawn_captures(list, the_pawns, dirs[i], target, seventh_rank);
            }
        }
    }
}
