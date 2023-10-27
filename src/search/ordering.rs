use crate::types::{Move, MoveList, Piece, MAX_MOVES};

impl<'a> super::Searcher<'a> {
    const CACHE_MOVE: i32 = 300_000_000;
    const MVV_LVA: i32 = 200_000_000;
    const KILLERS: i32 = 100_000_000;

    /// Returns an array of move ratings for the specified move list.
    pub fn build_ordering(&self, moves: &MoveList, cache_move: Option<Move>) -> [i32; MAX_MOVES] {
        let mut ordering = [0; MAX_MOVES];
        for index in 0..moves.length() {
            ordering[index] = self.get_move_rating(moves[index], cache_move);
        }
        ordering
    }

    /// Returns the rating of the specified move.
    fn get_move_rating(&self, mv: Move, cache_move: Option<Move>) -> i32 {
        if Some(mv) == cache_move {
            return Self::CACHE_MOVE;
        }
        if mv.is_capture() {
            return self.mvv_lva(mv);
        }
        if self.killers.contains(mv, self.board.ply) {
            return Self::KILLERS;
        }
        self.history.get(mv)
    }

    /// Returns the Most Valuable Victim - Least Valuable Attacker score for the specified move.
    fn mvv_lva(&self, mv: Move) -> i32 {
        let attacker = self.board.get_piece(mv.start()).unwrap();
        // Handles en passant captures, assuming the victim is a pawn if the target is empty
        let victim = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
        Self::MVV_LVA + victim as i32 * 10 - attacker as i32
    }
}
