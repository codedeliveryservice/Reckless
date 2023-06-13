use game::{Move, Piece};

use super::alphabeta::AlphaBetaSearch;

use self::OrderingStage::*;

const NORMAL_STAGES: &[OrderingStage] = &[CacheMove, MvvLva, Killer, History];
const QUIESCENCE_STAGES: &[OrderingStage] = &[MvvLva];

enum OrderingStage {
    CacheMove,
    MvvLva,
    Killer,
    History,
}

/// Container for the ordering of moves to be searched.
pub struct Ordering {
    items: Vec<(Move, u16)>,
    index: usize,
}

impl Ordering {
    /// Returns the next highest rated `Move` or `None` if there are no moves left.
    pub fn next(&mut self) -> Option<Move> {
        if self.index == self.items.len() {
            return None;
        }

        // Compare the current move rating with all others and swap if it's lower
        for next in (self.index + 1)..self.items.len() {
            if self.items[self.index].1 < self.items[next].1 {
                self.items.swap(self.index, next);
            }
        }

        let best = self.items[self.index].0;
        self.index += 1;
        Some(best)
    }
}

impl<'a> AlphaBetaSearch<'a> {
    /// Move from TT is likely to be the best and should be rated higher all others.
    const TT_MOVE: u16 = 2000;
    /// Most Valuable Victim – Least Valuable Attacker heuristic table indexed by `[attacker][victim]`.
    const MVV_LVA: [[u16; Piece::NUM]; Piece::NUM] = [
        [1015, 1025, 1035, 1045, 1055, 1065],
        [1014, 1024, 1034, 1044, 1054, 1064],
        [1013, 1023, 1033, 1043, 1053, 1063],
        [1012, 1022, 1032, 1042, 1052, 1062],
        [1011, 1021, 1031, 1041, 1051, 1061],
        [1010, 1020, 1030, 1040, 1050, 1060],
    ];
    /// Moves that caused a beta cutoff in the previous search.
    const KILLER_MOVE: u16 = 1000;

    /// Builds the ordering of moves to be searched in the normal search.
    pub(super) fn build_normal_ordering(&self) -> Ordering {
        let cache = &self.thread.cache.lock().unwrap();
        let cache_move = cache.read(self.board.hash, self.ply).map(|e| e.best);
        self.build_ordering(NORMAL_STAGES, cache_move)
    }

    /// Builds the ordering of moves to be searched in the quiescence search.
    pub(super) fn build_quiescence_ordering(&self) -> Ordering {
        self.build_ordering(QUIESCENCE_STAGES, None)
    }

    /// Builds the ordering of moves to be searched based on the given stages.
    fn build_ordering(&self, stages: &[OrderingStage], cache_move: Option<Move>) -> Ordering {
        let moves = self.board.generate_moves();
        let mut items = Vec::with_capacity(moves.len());
        for mv in moves {
            let rating = self.get_move_rating(mv, stages, cache_move);
            items.push((mv, rating));
        }

        Ordering { items, index: 0 }
    }

    /// Compute a rating for the specified move based on the given stages.
    fn get_move_rating(&self, mv: Move, stages: &[OrderingStage], cache_move: Option<Move>) -> u16 {
        for stage in stages {
            match stage {
                OrderingStage::CacheMove => {
                    if Some(mv) == cache_move {
                        return Self::TT_MOVE;
                    }
                }
                OrderingStage::MvvLva => {
                    if mv.is_capture() {
                        let start = self.board.get_piece(mv.start()).unwrap();
                        // Handles en passant captures by treating the default piece as a pawn since
                        // the target square of the capturing piece is different from the move's target square
                        let target = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
                        return Self::MVV_LVA[start][target];
                    }
                }
                OrderingStage::Killer => {
                    if self.thread.killers.contains(mv, self.ply) {
                        return Self::KILLER_MOVE;
                    }
                }
                OrderingStage::History => {
                    return self.thread.history.get_score(mv.start(), mv.target());
                }
            }
        }

        Default::default()
    }
}
