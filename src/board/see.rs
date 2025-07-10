use crate::{
    lookup::{bishop_attacks, rook_attacks},
    parameters::PIECE_VALUES,
    types::{Bitboard, Move, PieceType},
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
        balance -= PIECE_VALUES[self.piece_on(mv.from()).piece_type()];
        if balance >= 0 {
            return true;
        }

        let mut occupancies = self.occupancies();
        occupancies.clear(mv.from());
        occupancies.set(mv.to());

        let mut attackers = self.attackers_to(mv.to(), occupancies) & occupancies;
        let mut stm = !self.side_to_move();

        let diagonal = self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen);
        let orthogonal = self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen);

        loop {
            let our_attackers = attackers & self.colors(stm);
            if our_attackers.is_empty() {
                break;
            }

            let attacker = self.least_valuable_attacker(our_attackers);

            // The king cannot capture a protected piece; the side to move loses the exchange
            if attacker == PieceType::King && !(attackers & self.colors(!stm)).is_empty() {
                break;
            }

            // Make the capture
            occupancies.clear((self.pieces(attacker) & our_attackers).lsb());
            stm = !stm;

            // Assume our piece is going to be captured
            balance = -balance - 1 - PIECE_VALUES[attacker];
            if balance >= 0 {
                break;
            }

            // Capturing a piece may reveal a new sliding attacker
            if [PieceType::Pawn, PieceType::Bishop, PieceType::Queen].contains(&attacker) {
                attackers |= bishop_attacks(mv.to(), occupancies) & diagonal;
            }
            if [PieceType::Rook, PieceType::Queen].contains(&attacker) {
                attackers |= rook_attacks(mv.to(), occupancies) & orthogonal;
            }
            attackers &= occupancies;
        }

        // The last side to move has failed to capture back
        // since it has no more attackers and, therefore, is losing
        stm != self.side_to_move()
    }

    fn move_value(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return PIECE_VALUES[PieceType::Pawn];
        }

        let capture = self.piece_on(mv.to()).piece_type();
        PIECE_VALUES[capture]
    }

    fn least_valuable_attacker(&self, attackers: Bitboard) -> PieceType {
        for index in 0..PieceType::NUM {
            let piece = PieceType::new(index);
            if !(self.pieces(piece) & attackers).is_empty() {
                return piece;
            }
        }
        unreachable!();
    }
}
