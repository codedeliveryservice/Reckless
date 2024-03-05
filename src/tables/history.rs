use crate::types::{FullMove, Move, Piece, Square};

const MAX_HISTORY: i32 = 512;

/// Returns the bonus for a move based on the depth of the search.
fn bonus(depth: i32) -> i32 {
    depth * depth
}

/// Updates the score of an entry using a gravity function.
fn update<const IS_GOOD: bool>(v: &mut i32, depth: i32) {
    let bonus = bonus(depth);
    if IS_GOOD {
        *v += bonus - bonus * *v / MAX_HISTORY;
    } else {
        *v -= bonus + bonus * *v / MAX_HISTORY;
    }
}

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
pub struct HistoryMoves {
    table: [[i32; Square::NUM]; Square::NUM],
}

impl HistoryMoves {
    /// Returns the score of a move.
    pub fn get(&self, mv: Move) -> i32 {
        self.table[mv.start()][mv.target()]
    }

    /// Increases the score of a move based on the depth of the search.
    pub fn increase(&mut self, mv: Move, depth: i32) {
        update::<true>(&mut self.table[mv.start()][mv.target()], depth);
    }

    /// Decreases the score of a move based on the depth of the search.
    pub fn decrease(&mut self, mv: Move, depth: i32) {
        update::<false>(&mut self.table[mv.start()][mv.target()], depth);
    }
}

impl Default for HistoryMoves {
    fn default() -> Self {
        Self {
            table: [[0; Square::NUM]; Square::NUM],
        }
    }
}

pub struct ContinuationHistory {
    table: [[[[i32; Square::NUM]; Piece::NUM]; Square::NUM]; Piece::NUM],
}

impl ContinuationHistory {
    pub fn new() -> Box<Self> {
        unsafe {
            let layout = std::alloc::Layout::new::<Self>();
            let ptr = std::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr.cast())
        }
    }

    pub fn get(&self, previous: FullMove, current: FullMove) -> i32 {
        self.table[previous.piece()][previous.target()][current.piece()][current.target()]
    }

    pub fn increase(&mut self, previous: FullMove, current: FullMove, depth: i32) {
        let entry = &mut self.table[previous.piece()][previous.target()][current.piece()][current.target()];
        update::<true>(entry, depth);
    }

    pub fn decrease(&mut self, previous: FullMove, current: FullMove, depth: i32) {
        let entry = &mut self.table[previous.piece()][previous.target()][current.piece()][current.target()];
        update::<false>(entry, depth);
    }
}
