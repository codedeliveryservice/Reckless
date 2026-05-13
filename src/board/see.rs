use crate::{
    lookup::{bishop_attacks, ray_pass, rook_attacks},
    types::{Bitboard, Color, Move, PieceType},
};

impl super::Board {
    /// Checks if the static exchange evaluation (SEE) of a move meets the given `threshold`,
    /// indicating that the sequence of captures on a single square, starting with the move,
    /// results in a value greater than or equal to the threshold for the side to move.
    ///
    /// Promotions and castling always pass this check.
    pub fn see(&self, mv: Move, threshold: i32) -> bool {
        if mv.is_castling() {
            return true;
        }

        // In the best case, we win a piece, but still end up with a negative balance
        let mut balance = self.move_value(mv) - threshold;
        if balance < 0 {
            return false;
        }

        // In the worst case, we lose a piece, but still end up with a non-negative balance
        balance -= if mv.is_promotion() { mv.promo_piece_type().value() } else { self.piece_on(mv.from()).value() };

        if balance >= 0 {
            return true;
        }

        // No need to set the to square for SEE
        let mut occupancies = self.occupancies();
        occupancies.clear(mv.from());

        if mv.is_en_passant() {
            occupancies.clear(mv.to() ^ 8);
        }

        let mut attackers = self.attackers_to(mv.to(), occupancies) & occupancies;
        let mut stm = !self.side_to_move();

        let diagonal = self.pieces2(PieceType::Bishop, PieceType::Queen);
        let orthogonal = self.pieces2(PieceType::Rook, PieceType::Queen);

        let king_rays =
            [ray_pass(self.king_square(Color::White), mv.to()), ray_pass(self.king_square(Color::Black), mv.to())];

        loop {
            let mut our_attackers = attackers & self.colors(stm);

            // Exclude pinned pieces if pinners are still on the board
            if (self.pinners(!stm) & occupancies) != Bitboard(0) {
                our_attackers &= !(self.pinned(stm) & !king_rays[stm]);
            }

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
            balance = -balance - 1 - attacker.value();
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
        let capture = self.type_on(mv.capture_sq());
        let mut value = capture.value();

        if mv.is_promotion() {
            value += mv.promo_piece_type().value() - PieceType::Pawn.value()
        }
        value
    }

    #[cfg(target_feature = "avx512f")]
    fn least_valuable_attacker(&self, attackers: Bitboard) -> PieceType {
        use std::arch::x86_64::*;

        let overlaps = unsafe {
            let pieces = _mm512_setr_epi64(
                (self.pieces(PieceType::Pawn) & attackers).0 as i64,
                (self.pieces(PieceType::Knight) & attackers).0 as i64,
                (self.pieces(PieceType::Bishop) & attackers).0 as i64,
                (self.pieces(PieceType::Rook) & attackers).0 as i64,
                (self.pieces(PieceType::Queen) & attackers).0 as i64,
                (self.pieces(PieceType::King) & attackers).0 as i64,
                0,
                0,
            );
            _mm512_test_epi64_mask(pieces, pieces) as u8
        };

        let mask = overlaps & 0x3F;

        debug_assert_ne!(mask, 0, "least_valuable_attacker called with empty attackers bitboard");

        PieceType::new(mask.trailing_zeros() as usize)
    }

    #[cfg(not(target_feature = "avx512f"))]
    fn least_valuable_attacker(&self, attackers: Bitboard) -> PieceType {
        let mask = u8::from(!(self.pieces(PieceType::Pawn) & attackers).is_empty())
            | u8::from(!(self.pieces(PieceType::Knight) & attackers).is_empty()) << 1
            | u8::from(!(self.pieces(PieceType::Bishop) & attackers).is_empty()) << 2
            | u8::from(!(self.pieces(PieceType::Rook) & attackers).is_empty()) << 3
            | u8::from(!(self.pieces(PieceType::Queen) & attackers).is_empty()) << 4
            | u8::from(!(self.pieces(PieceType::King) & attackers).is_empty()) << 5;

        debug_assert_ne!(mask, 0, "least_valuable_attacker called with empty attackers bitboard");

        PieceType::new(mask.trailing_zeros() as usize)
    }
}
