use crate::{
    lookup::{bishop_attacks, rook_attacks},
    parameters::PIECE_VALUES,
    types::{Bitboard, Move, Piece},
};

impl super::Board {
    /// Checks if the static exchange evaluation (SEE) of a move meets the `threshold`,
    /// indicating that the sequence of captures on a single square, starting with the move,
    /// results in a non-negative balance for the side to move.
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
        balance -= PIECE_VALUES[self.piece_on(mv.start())];
        if balance >= 0 {
            return true;
        }

        let mut occupancies = self.occupancies();
        occupancies.clear(mv.start());
        occupancies.set(mv.target());

        let mut attackers = self.attackers_to(mv.target(), occupancies) & occupancies;
        let mut stm = !self.side_to_move();

        let diagonal = self.pieces(Piece::Bishop) | self.pieces(Piece::Queen);
        let orthogonal = self.pieces(Piece::Rook) | self.pieces(Piece::Queen);

        loop {
            let our_attackers = attackers & self.colors(stm);
            if our_attackers.is_empty() {
                break;
            }

            let attacker = self.least_valuable_attacker(our_attackers);

            // The king cannot capture a protected piece; the side to move loses the exchange
            if attacker == Piece::King && !(attackers & self.colors(!stm)).is_empty() {
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
            if [Piece::Pawn, Piece::Bishop, Piece::Queen].contains(&attacker) {
                attackers |= bishop_attacks(mv.target(), occupancies) & diagonal;
            }
            if [Piece::Rook, Piece::Queen].contains(&attacker) {
                attackers |= rook_attacks(mv.target(), occupancies) & orthogonal;
            }
            attackers &= occupancies;
        }

        // The last side to move has failed to capture back
        // since it has no more attackers and, therefore, is losing
        stm != self.side_to_move()
    }

    fn move_value(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return PIECE_VALUES[Piece::Pawn];
        }

        let capture = self.piece_on(mv.target());
        PIECE_VALUES[capture]
    }

    fn least_valuable_attacker(&self, attackers: Bitboard) -> Piece {
        for index in 0..Piece::NUM {
            let piece = Piece::new(index);
            if !(self.pieces(piece) & attackers).is_empty() {
                return piece;
            }
        }
        unreachable!();
    }
}
