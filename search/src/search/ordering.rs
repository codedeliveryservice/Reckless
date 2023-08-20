use game::{Move, Piece};

use super::AlphaBetaSearch;

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
    const CACHE_MOVE: u16 = 3000;
    /// Most Valuable Victim – Least Valuable Attacker heuristic.
    const MVV_LVA: u16 = 2000;
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
            return match stage {
                OrderingStage::CacheMove if Some(mv) == cache_move => Self::CACHE_MOVE,
                OrderingStage::MvvLva if mv.is_capture() => {
                    let attacker = self.board.get_piece(mv.start()).unwrap();
                    // Handles en passant captures, assuming the victim is a pawn if the target is empty
                    let victim = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
                    Self::MVV_LVA + victim as u16 * 10 - attacker as u16
                }
                OrderingStage::Killer if self.killers.contains(mv, self.ply) => Self::KILLER_MOVE,
                OrderingStage::History => self.history.get_score(mv.start(), mv.target()),
                _ => continue,
            };
        }
        Default::default()
    }
}
