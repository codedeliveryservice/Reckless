use game::{Move, MoveList, Piece};

use super::AlphaBetaSearch;

use self::OrderingStage::*;

pub const ALPHABETA_STAGES: &[OrderingStage] = &[CacheMove, MvvLva, Killer, History];
pub const QUIESCENCE_STAGES: &[OrderingStage] = &[MvvLva];

pub enum OrderingStage {
    CacheMove,
    MvvLva,
    Killer,
    History,
}

impl<'a> AlphaBetaSearch<'a> {
    const CACHE_MOVE: u32 = 3_000_000;
    const MVV_LVA: u32 = 2_000_000;
    const KILLERS: u32 = 1_000_000;

    /// Builds the ordering of moves to be searched based on the given stages.
    pub fn build_ordering(&self, stages: &[OrderingStage], moves: &MoveList, cache_move: Option<Move>) -> Vec<u32> {
        let mut ordering = Vec::with_capacity(moves.length());
        for mv in moves.iter() {
            ordering.push(self.get_move_rating(mv, stages, cache_move));
        }
        ordering
    }

    /// Compute a rating for the specified move based on the given stages.
    fn get_move_rating(&self, mv: Move, stages: &[OrderingStage], cache_move: Option<Move>) -> u32 {
        for stage in stages {
            return match stage {
                CacheMove if Some(mv) == cache_move => Self::CACHE_MOVE,
                MvvLva if mv.is_capture() => self.mvv_lva(mv),
                Killer if self.killers.contains(mv, self.board.ply) => Self::KILLERS,
                History => self.history.get_score(mv),
                _ => continue,
            };
        }
        Default::default()
    }

    /// Returns the Most Valuable Victim - Least Valuable Attacker score for the specified move.
    pub(self) fn mvv_lva(&self, mv: Move) -> u32 {
        let attacker = self.board.get_piece(mv.start()).unwrap();
        // Handles en passant captures, assuming the victim is a pawn if the target is empty
        let victim = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
        Self::MVV_LVA + victim as u32 * 10 - attacker as u32
    }
}
