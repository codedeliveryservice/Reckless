use game::Move;

use super::{mvv_lva, SearchParams, SearchThread};

pub struct Ordering {
    items: Vec<(Move, u32)>,
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

/// Returns a move score based on heuristic analysis.
fn score_move(mv: Move, p: &SearchParams, thread: &SearchThread, tt_move: Option<Move>) -> u32 {
    if Some(mv) == tt_move {
        return 700;
    }

    if mv.is_capture() {
        return mvv_lva::score_mvv_lva(p.board, mv);
    }

    if thread.killers.contains(mv, p.ply) {
        // The quiet move score is rated below any capture move
        return 90;
    }

    Default::default()
}
