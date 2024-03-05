use crate::types::{FullMove, Move, MoveList, Piece, MAX_MOVES};

impl super::Searcher<'_> {
    const TT_MOVE: i32 = 400_000_000;
    const MVV_LVA: i32 = 300_000_000;
    const KILLERS: i32 = 200_000_000;
    const COUNTER: i32 = 100_000_000;

    /// Returns an array of move ratings for the specified move list.
    pub fn build_ordering(&self, moves: &MoveList, tt_move: Option<Move>) -> [i32; MAX_MOVES] {
        let counter = self.counters.get(self.board.side_to_move, self.board.last_move());
        let countermove = self.board.tail_move(1);
        let followup = self.board.tail_move(2);

        let mut ordering = [0; MAX_MOVES];
        for index in 0..moves.length() {
            ordering[index] = self.get_move_rating(moves[index], tt_move, counter, countermove, followup);
        }
        ordering
    }

    /// Returns the rating of the specified move.
    fn get_move_rating(
        &self,
        mv: Move,
        tt_move: Option<Move>,
        counter: Option<Move>,
        countermove: Option<FullMove>,
        followup: Option<FullMove>,
    ) -> i32 {
        if Some(mv) == tt_move {
            return Self::TT_MOVE;
        }
        if mv.is_capture() {
            return self.mvv_lva(mv);
        }
        if self.killers.contains(mv, self.board.ply) {
            return Self::KILLERS;
        }
        if Some(mv) == counter {
            return Self::COUNTER;
        }

        let piece = self.board.get_piece(mv.start()).unwrap();
        let current = FullMove::new(piece, mv);

        let mut score = self.history.get_main(mv);
        if let Some(previous) = countermove {
            score += self.history.get_countermove(previous, current);
        }
        if let Some(previous) = followup {
            score += self.history.get_followup(previous, current);
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
