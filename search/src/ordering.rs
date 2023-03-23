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

/// Returns a move score based on heuristic analysis.
///
/// Order of importance:
/// 1. Move from TT
/// 2. Capture move from MVV-LVA (best captures first)
/// 3. Killer move
/// 4. History move
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

    thread.history.get_score(mv.start(), mv.target())
}

/// Scores capture move based on the MVV LVA heuristic.
fn score_mvv_lva(board: &Board, mv: Move) -> Rating {
    let start = board.get_piece(mv.start()).unwrap();

    // This trick handles en passant captures by unwrapping as a pawn for a default piece,
    // since the target square for en passant is different from the captured piece's square
    let target = board.get_piece(mv.target()).unwrap_or(Piece::Pawn);

    MVV_LVA[start][target]
}
