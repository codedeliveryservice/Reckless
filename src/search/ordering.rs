use crate::types::{FullMove, Move, MoveList, MAX_MOVES};

impl super::SearchThread<'_> {
    const HASH_MOVE: i32 = 300_000_000;
    const GOOD_NOISY: i32 = 200_000_000;
    const KILLERS: i32 = 100_000_000;
    const BAD_NOISY: i32 = -100_000_000;

    /// Returns an array of move ratings for the specified move list.
    pub fn build_ordering(&self, moves: &MoveList, tt_move: Option<Move>) -> [i32; MAX_MOVES] {
        let continuations = [1, 2].map(|ply| self.board.tail_move(ply));
        let mut ordering = [0; MAX_MOVES];
        for index in 0..moves.len() {
            ordering[index] = self.get_move_rating(moves[index], tt_move, &continuations);
        }
        ordering
    }

    /// Returns the rating of the specified move.
    fn get_move_rating(&self, mv: Move, tt_move: Option<Move>, continuations: &[FullMove; 2]) -> i32 {
        if Some(mv) == tt_move {
            return Self::HASH_MOVE;
        }
        if mv.is_capture() {
            let base = if self.see(mv, 0) { Self::GOOD_NOISY } else { Self::BAD_NOISY };
            return base + self.mvv_lva(mv);
        }
        if self.killers[self.ply] == mv {
            return Self::KILLERS;
        }

        let piece = self.board.piece_on(mv.start());
        self.history.get_main(self.board.side_to_move(), mv)
            + self.history.get_continuation(0, continuations[0], piece, mv)
            + self.history.get_continuation(1, continuations[1], piece, mv)
    }

    /// Returns the Most Valuable Victim - Least Valuable Attacker score for the specified move.
    fn mvv_lva(&self, mv: Move) -> i32 {
        if mv.is_en_passant() {
            return 0;
        }

        let attacker = self.board.piece_on(mv.start()) as i32;
        let victim = self.board.piece_on(mv.target()) as i32;
        10 * victim - attacker
    }
}
