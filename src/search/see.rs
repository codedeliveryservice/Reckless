use crate::{
    lookup::*,
    types::{Bitboard, Color, Move, Piece, Square},
};

const PIECE_VALUES: [i32; Piece::NUM] = [100, 400, 400, 650, 1200, 0];

impl super::SearchThread<'_> {
    /// Returns `true` if the static exchange evaluation of the given move
    /// is greater than the given threshold. It means that the sequence of
    /// captures starting with the given move is winning for the side to move.
    pub fn see(&mut self, mv: Move, threshold: i32) -> bool {
        // The best case is a free capture or a safe promotion
        let mut balance = self.move_value(mv);

        // The best case is still losing
        if balance < threshold {
            return false;
        }

        // The worst case is losing our piece
        balance -= match mv.get_promotion_piece() {
            Some(promotion) => PIECE_VALUES[promotion],
            None => PIECE_VALUES[self.board.get_piece(mv.start()).unwrap()],
        };

        // The worst case is still winning
        if balance >= threshold {
            return true;
        }

        let mut occupancies = self.board.occupancies();
        // The moving piece is no longer on the board
        occupancies.clear(mv.start());

        let mut attackers = self.attackers_to(mv.target(), occupancies) & occupancies;
        let mut stm = !self.board.side_to_move;

        let diagonal = self.board.pieces(Piece::Bishop) | self.board.pieces(Piece::Queen);
        let orthogonal = self.board.pieces(Piece::Rook) | self.board.pieces(Piece::Queen);

        loop {
            let out_attackers = attackers & self.board.colors(stm);
            if out_attackers.is_empty() {
                break;
            }

            let attacker = self.least_valuable_attacker(out_attackers);

            // The exchange is losing, since we cannot capture a protected piece with the king
            if attacker == Piece::King && !(attackers & self.board.colors(!stm)).is_empty() {
                break;
            }

            // Make the capture
            occupancies.clear((self.board.pieces(attacker) & out_attackers).lsb());
            stm = !stm;

            // Assume our piece is going to be captured
            balance = -balance - 1 - PIECE_VALUES[attacker];
            if balance >= threshold {
                break;
            }

            // Capturing a piece may reveal a new sliding attacker
            if [Piece::Pawn, Piece::Bishop, Piece::Queen].contains(&attacker) {
                attackers |= bishop_attacks(mv.target(), occupancies) & diagonal;
            }
            if [Piece::Pawn, Piece::Rook, Piece::Queen].contains(&attacker) {
                attackers |= rook_attacks(mv.target(), occupancies) & orthogonal;
            }
            attackers &= occupancies;
        }

        // The last side to move has failed to capture back
        // since it has no more attackers and, therefore, is losing
        stm != self.board.side_to_move
    }

    fn move_value(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return PIECE_VALUES[Piece::Pawn];
        }

        let capture = self.board.get_piece(mv.target()).unwrap();
        PIECE_VALUES[capture]
    }

    fn least_valuable_attacker(&mut self, our_attackers: Bitboard) -> Piece {
        for index in 0..Piece::NUM {
            let piece = Piece::new(index);
            let piece_bb = self.board.pieces(piece) & our_attackers;
            if !piece_bb.is_empty() {
                return piece;
            }
        }
        panic!("There should be at least one attacker");
    }

    fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        let bishop_or_queen = self.board.pieces(Piece::Bishop) | self.board.pieces(Piece::Queen);
        let rook_or_queen = self.board.pieces(Piece::Rook) | self.board.pieces(Piece::Queen);

        king_attacks(square) & self.board.pieces(Piece::King)
            | knight_attacks(square) & self.board.pieces(Piece::Knight)
            | pawn_attacks(square, Color::White) & self.board.of(Piece::Pawn, Color::Black)
            | pawn_attacks(square, Color::Black) & self.board.of(Piece::Pawn, Color::White)
            | rook_attacks(square, occupancies) & rook_or_queen
            | bishop_attacks(square, occupancies) & bishop_or_queen
    }
}
