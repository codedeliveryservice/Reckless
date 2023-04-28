use game::{Board, Move, Piece};

use self::OrderingStage::*;
use super::SearchThread;

type Rating = u16;
type OrderingMap = Vec<(Move, Rating)>;

const NORMAL_STAGES: &[OrderingStage] = &[CacheMove, MvvLva, Killer, History];
const QUIESCENCE_STAGES: &[OrderingStage] = &[MvvLva];

enum OrderingStage {
    CacheMove,
    MvvLva,
    Killer,
    History,
}

pub struct Ordering {
    items: OrderingMap,
    index: usize,
}

impl Ordering {
    pub fn normal(board: &Board, ply: usize, thread: &SearchThread) -> Self {
        let hash = board.hash_key;
        let cache_move = thread.cache.lock().unwrap().read(hash).map(|e| e.best);
        Self::build(NORMAL_STAGES, board, ply, thread, cache_move)
    }

    pub fn quiescence(board: &Board, ply: usize, thread: &SearchThread) -> Self {
        Self::build(QUIESCENCE_STAGES, board, ply, thread, None)
    }

    /// Builds a new `Ordering` object sorted by rating for a given set of stages.
    fn build(
        stages: &[OrderingStage],
        board: &Board,
        ply: usize,
        thread: &SearchThread,
        cache_move: Option<Move>,
    ) -> Self {
        let builder = OrderingBuilder {
            board,
            ply,
            thread,
            cache_move,
        };

        Self {
            items: builder.build(stages),
            index: 0,
        }
    }

    /// Returns the next most rated `Move` or `None` if there are no moves left.
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

struct OrderingBuilder<'a> {
    board: &'a Board,
    ply: usize,
    thread: &'a SearchThread,
    cache_move: Option<Move>,
}

impl<'a> OrderingBuilder<'a> {
    /// Move from TT is likely to be the best and should be rated higher all others
    const TT_MOVE: Rating = 2000;

    /// Quiet killer move is rated below any capture move from MVV-LVA
    const KILLER_MOVE: Rating = 1000;

    /// Most Valuable Victim – Least Valuable Attacker heuristic table indexed by `[attacker][victim]`.
    const MVV_LVA: [[Rating; Piece::NUM]; Piece::NUM] = [
        [1015, 1025, 1035, 1045, 1055, 1065],
        [1014, 1024, 1034, 1044, 1054, 1064],
        [1013, 1023, 1033, 1043, 1053, 1063],
        [1012, 1022, 1032, 1042, 1052, 1062],
        [1011, 1021, 1031, 1041, 1051, 1061],
        [1010, 1020, 1030, 1040, 1050, 1060],
    ];

    pub fn build(self, stages: &[OrderingStage]) -> OrderingMap {
        let moves = self.board.generate_moves();
        let mut map = Vec::with_capacity(moves.len());
        for mv in moves {
            let rating = self.get_move_rating(stages, mv);
            map.push((mv, rating));
        }
        map
    }

    /// Compute a rating for the specified move based on the given stages.
    fn get_move_rating(&self, stages: &[OrderingStage], mv: Move) -> u16 {
        for stage in stages {
            match stage {
                CacheMove => {
                    if Some(mv) == self.cache_move {
                        return Self::TT_MOVE;
                    }
                }
                MvvLva => {
                    if let Some(value) = self.get_mvv_lva_rating(mv) {
                        return value;
                    }
                }
                Killer => {
                    if self.thread.killers.contains(mv, self.ply) {
                        return Self::KILLER_MOVE;
                    }
                }
                History => {
                    return self.thread.history.get_score(mv.start(), mv.target());
                }
            }
        }

        Default::default()
    }

    /// Compute the MVV/LVA rating for the specified move, if it is a capture.
    ///
    /// If the move is not a capture, `None` is returned.
    fn get_mvv_lva_rating(&self, mv: Move) -> Option<Rating> {
        if !mv.is_capture() {
            return None;
        }

        let start = self.board.get_piece(mv.start()).unwrap();
        // This trick handles en passant captures by unwrapping as a pawn for a default piece,
        // since the target square for en passant is different from the captured piece's square
        let target = self.board.get_piece(mv.target()).unwrap_or(Piece::Pawn);
        Some(Self::MVV_LVA[start][target])
    }
}
