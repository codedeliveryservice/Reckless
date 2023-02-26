use game::{Board, Move, Piece};

use super::{SearchParams, SearchThread};

type Rating = u16;

pub struct Ordering {
    items: Vec<(Move, Rating)>,
    index: usize,
}

impl Ordering {
    /// Creates a rated list of moves and returns `Ordering` wrapper over it.
    pub fn generate(p: &SearchParams, thread: &SearchThread, tt_move: Option<Move>) -> Self {
        let moves = p.board.generate_moves();
        let mut items = Vec::with_capacity(moves.len());
        for mv in moves {
            items.push((mv, score_move(mv, p, thread, tt_move)));
        }

        Self { items, index: 0 }
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

/// Move from TT is likely to be the best and should be rated higher all others
const TT_MOVE: Rating = 700;

/// Quiet killer move is rated below any capture move from MVV-LVA
const KILLER_MOVE: Rating = 90;

/// Most Valuable Victim – Least Valuable Attacker heuristic table indexed by `[attacker][victim]`.
const MVV_LVA: [[Rating; Piece::NUM]; Piece::NUM] = [
    [105, 205, 305, 405, 505, 605],
    [104, 204, 304, 404, 504, 604],
    [103, 203, 303, 403, 503, 603],
    [102, 202, 302, 402, 502, 602],
    [101, 201, 301, 401, 501, 601],
    [100, 200, 300, 400, 500, 600],
];

/// Returns a move score based on heuristic analysis.
fn score_move(mv: Move, p: &SearchParams, thread: &SearchThread, tt_move: Option<Move>) -> Rating {
    if Some(mv) == tt_move {
        return TT_MOVE;
    }

    if mv.is_capture() {
        return score_mvv_lva(p.board, mv);
    }

    if thread.killers.contains(mv, p.ply) {
        return KILLER_MOVE;
    }

    Default::default()
}

/// Scores capture move based on the MVV LVA heuristic.
fn score_mvv_lva(board: &Board, mv: Move) -> Rating {
    let start = board.get_piece(mv.start()).unwrap();

    // This trick handles en passant captures by unwrapping as a pawn for a default piece,
    // since the target square for en passant is different from the captured piece's square
    let target = board.get_piece(mv.target()).unwrap_or(Piece::Pawn);

    MVV_LVA[start][target]
}
