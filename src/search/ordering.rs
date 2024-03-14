use crate::types::{FullMove, Move, MoveList, Piece, MAX_MOVES};

impl super::Searcher<'_> {
    const TT_MOVE: i32 = 300_000_000;
    const MVV_LVA: i32 = 200_000_000;
    const KILLERS: i32 = 100_000_000;

    /// Returns an array of move ratings for the specified move list.
    pub fn build_ordering(&self, moves: &MoveList, tt_move: Option<Move>) -> [i32; MAX_MOVES] {
        let continuations = [1, 2].map(|ply| self.board.tail_move(ply));
        let mut ordering = [0; MAX_MOVES];
        for index in 0..moves.length() {
            ordering[index] = self.get_move_rating(moves[index], tt_move, &continuations);
        }
        ordering
    }

    /// Returns the rating of the specified move.
    fn get_move_rating(&self, mv: Move, tt_move: Option<Move>, continuations: &[FullMove]) -> i32 {
        if Some(mv) == tt_move {
            return Self::TT_MOVE;
        }
        if mv.is_capture() {
            return self.mvv_lva(mv);
        }
        if self.killers.contains(mv, self.board.ply) {
            return Self::KILLERS;
        }

        let piece = self.board.get_piece(mv.start()).unwrap();

        let mut score = self.history.get_main(self.board.side_to_move, mv);
        for (kind, &previous) in continuations.into_iter().enumerate() {
            score += self.history.get_continuation(kind, previous, piece, mv);
        }
        score
    }

    /// Returns the Most Valuable Victim - Least Valuable Attacker score for the specified move.
    fn mvv_lva(&self, mv: Move) -> i32 {
        let attacker = self.board.get_piece(mv.start()).unwrap();
        // Handles en passant captures, assuming the victim is a pawn if the target is empty
        let victim = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
        Self::MVV_LVA + victim as i32 * 10 - attacker as i32
    }
}
