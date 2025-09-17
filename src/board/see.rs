use crate::{
    lookup::{bishop_attacks, rook_attacks},
    parameters::PIECE_VALUES,
    types::{Move, PieceType},
};

impl super::Board {
    /// Checks if the static exchange evaluation (SEE) of a move meets the given `threshold`,
    /// indicating that the sequence of captures on a single square, starting with the move,
    /// results in a value greater than or equal to the threshold for the side to move.
    ///
    /// Promotions and castling always pass this check.
    pub fn see(&self, mv: Move, threshold: i32) -> bool {
        if mv.is_promotion() || mv.is_castling() {
            return true;
        }

        // In the best case, we win a piece, but still end up with a negative balance
        let mut balance = self.move_value(mv) - threshold;
        if balance < 0 {
            return false;
        }

        // In the worst case, we lose a piece, but still end up with a non-negative balance
        balance = PIECE_VALUES[self.piece_on(mv.from()).piece_type()] - balance;
        if balance <= 0 {
            return true;
        }

        let mut occupancies = self.occupancies();
        occupancies.clear(mv.from());
        occupancies.set(mv.to());

        if mv.is_en_passant() {
            occupancies.clear(mv.to() ^ 8);
        }

        let mut attackers = self.attackers_to(mv.to(), occupancies) & occupancies;
        let mut stm = self.side_to_move();

        let diagonal = self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen);
        let orthogonal = self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen);

        let mut result = 1;

        loop {
            stm = !stm;
            attackers &= occupancies;

            let our_attackers = attackers & self.colors(stm);
            if our_attackers.is_empty() {
                break;
            }

            result ^= 1;

            let bitboard = our_attackers & self.pieces(PieceType::Pawn);
            if !bitboard.is_empty() {
                balance = PIECE_VALUES[PieceType::Pawn] - balance;
                if balance < result {
                    break;
                }

                occupancies.clear(bitboard.lsb());
                attackers |= bishop_attacks(mv.to(), occupancies) & diagonal;
                continue;
            }

            let bitboard = our_attackers & self.pieces(PieceType::Knight);
            if !bitboard.is_empty() {
                balance = PIECE_VALUES[PieceType::Knight] - balance;
                if balance < result {
                    break;
                }

                occupancies.clear(bitboard.lsb());
                continue;
            }

            let bitboard = our_attackers & self.pieces(PieceType::Bishop);
            if !bitboard.is_empty() {
                balance = PIECE_VALUES[PieceType::Bishop] - balance;
                if balance < result {
                    break;
                }

                occupancies.clear(bitboard.lsb());
                attackers |= bishop_attacks(mv.to(), occupancies) & diagonal;
                continue;
            }

            let bitboard = our_attackers & self.pieces(PieceType::Rook);
            if !bitboard.is_empty() {
                balance = PIECE_VALUES[PieceType::Rook] - balance;
                if balance < result {
                    break;
                }

                occupancies.clear(bitboard.lsb());
                attackers |= rook_attacks(mv.to(), occupancies) & orthogonal;
                continue;
            }

            let bitboard = our_attackers & self.pieces(PieceType::Queen);
            if !bitboard.is_empty() {
                balance = PIECE_VALUES[PieceType::Queen] - balance;
                if balance < result {
                    break;
                }

                occupancies.clear(bitboard.lsb());
                attackers |= bishop_attacks(mv.to(), occupancies) & diagonal;
                attackers |= rook_attacks(mv.to(), occupancies) & orthogonal;
                continue;
            }

            return if (attackers & !self.colors(stm)).is_empty() { result != 0 } else { (result ^ 1) != 0 };
        }

        result != 0
    }

    fn move_value(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return PIECE_VALUES[PieceType::Pawn];
        }

        let capture = self.piece_on(mv.to()).piece_type();
        PIECE_VALUES[capture]
    }
}
