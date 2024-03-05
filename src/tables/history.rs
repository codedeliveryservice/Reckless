use crate::types::{FullMove, Move, Piece, Square};

const MAX_HISTORY: i32 = 512;

// [start][target]
type ButterflyHistory = [[i32; Square::NUM]; Square::NUM];
// [previous move piece][previous move target][current move piece][current move target]
type ContinuationHistory = [[[[i32; Square::NUM]; Piece::NUM]; Square::NUM]; Piece::NUM];

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
pub struct History {
    main: ButterflyHistory,
    /// Indexed by current move and opponent's last move (1 ply ago).
    countermove: ContinuationHistory,
    /// Indexed by current move and our previous move (2 plies ago).
    followup: ContinuationHistory,
}

impl History {
    /// Creates a new instance of the history heuristic.
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

    /// Returns the score of the main butterfly history heuristic.
    pub fn get_main(&self, mv: Move) -> i32 {
        self.main[mv.start()][mv.target()]
    }

    /// Returns the score of the countermove history heuristic.
    pub fn get_countermove(&self, previous: FullMove, current: FullMove) -> i32 {
        self.countermove[previous.piece()][previous.target()][current.piece()][current.target()]
    }

    /// Returns the score of the followup history heuristic.
    pub fn get_followup(&self, previous: FullMove, current: FullMove) -> i32 {
        self.followup[previous.piece()][previous.target()][current.piece()][current.target()]
    }

    /// Updates the main butterfly history heuristic.
    pub fn update_main<const IS_GOOD: bool>(&mut self, mv: Move, depth: i32) {
        update::<IS_GOOD>(&mut self.main[mv.start()][mv.target()], depth);
    }

    /// Updates the countermove history heuristic.
    pub fn update_countermove<const IS_GOOD: bool>(&mut self, previous: FullMove, current: FullMove, depth: i32) {
        let entry = &mut self.countermove[previous.piece()][previous.target()][current.piece()][current.target()];
        update::<IS_GOOD>(entry, depth);
    }

    /// Updates the followup history heuristic.
    pub fn update_followup<const IS_GOOD: bool>(&mut self, previous: FullMove, current: FullMove, depth: i32) {
        let entry = &mut self.followup[previous.piece()][previous.target()][current.piece()][current.target()];
        update::<IS_GOOD>(entry, depth);
    }
}

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
