use crate::{
    board::Board,
    types::{FullMove, Move, Piece, Square},
};

const MAX_HISTORY: i32 = 16384;

type Butterfly<T> = [[T; Square::NUM]; Square::NUM];
type PieceSquare<T> = [T; Square::NUM * (Piece::NUM + 1)];

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
pub struct History {
    main: Butterfly<i32>,
    continuations: [PieceSquare<PieceSquare<i32>>; 2],
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

    pub fn get_continuation(&self, index: usize, previous: FullMove, piece: Piece, current: Move) -> i32 {
        let previous = previous.piece() as usize * Square::NUM + previous.target() as usize;
        let current = piece as usize * Square::NUM + current.target() as usize;

        self.continuations[index][previous][current]
    }

    pub fn update_main(&mut self, mv: Move, fails: &[Move], depth: i32) {
        update::<true>(&mut self.main[mv.start()][mv.target()], depth);
        for &fail in fails {
            update::<false>(&mut self.main[fail.start()][fail.target()], depth);
        }
    }

    pub fn update_continuation(&mut self, board: &Board, current: Move, fails: &[Move], depth: i32) {
        let piece = board.get_piece(current.start()).unwrap();

        for (kind, ply) in [1, 2].into_iter().enumerate() {
            let previous = board.tail_move(ply);
            if previous == FullMove::NULL {
                continue;
            }

            update::<true>(self.get_continuation_mut(kind, previous, piece, current), depth);
            for &fail in fails {
                let piece = board.get_piece(fail.start()).unwrap();
                update::<false>(self.get_continuation_mut(kind, previous, piece, fail), depth);
            }
        }
    }

    fn get_continuation_mut(&mut self, index: usize, previous: FullMove, piece: Piece, current: Move) -> &mut i32 {
        let previous = previous.piece() as usize * Square::NUM + previous.target() as usize;
        let current = piece as usize * Square::NUM + current.target() as usize;

        &mut self.continuations[index][previous][current]
    }
}

/// Returns the bonus for a move based on the depth of the search.
fn bonus(depth: i32) -> i32 {
    (150 * depth - 25).min(1780)
}

/// Returns the malus for a move based on the depth of the search.
fn malus(depth: i32) -> i32 {
    (160 * depth + 15).min(1800)
}

/// Updates the score of an entry using a gravity function.
fn update<const IS_GOOD: bool>(v: &mut i32, depth: i32) {
    if IS_GOOD {
        let bonus = bonus(depth);
        *v += bonus - bonus * *v / MAX_HISTORY;
    } else {
        let malus = malus(depth);
        *v -= malus + malus * *v / MAX_HISTORY;
    }
}
