use crate::{
    board::Board,
    types::{Color, FullMove, Move, Piece, Square},
};

const MAX_HISTORY: i32 = 16384;

type Butterfly<T> = [[T; Square::NUM]; Square::NUM];
type PieceSquare<T> = [T; Square::NUM * (Piece::NUM + 1)];

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
#[derive(Clone)]
pub struct History {
    capture: Box<[Butterfly<[i32; Piece::NUM]>; Color::NUM]>,
    main: Box<[Butterfly<i32>; Color::NUM]>,
    continuations: Box<[PieceSquare<PieceSquare<i32>>; 2]>,
}

impl History {
    pub fn get_capture(&self, stm: Color, mv: Move, capture: Piece) -> i32 {
        self.capture[stm][mv.start()][mv.target()][capture]
    }

    pub fn get_main(&self, stm: Color, mv: Move) -> i32 {
        self.main[stm][mv.start()][mv.target()]
    }

    pub fn get_continuation(&self, index: usize, previous: FullMove, piece: Piece, current: Move) -> i32 {
        let previous = previous.piece() as usize * Square::NUM + previous.target() as usize;
        let current = piece as usize * Square::NUM + current.target() as usize;

        self.continuations[index][previous][current]
    }

    pub fn update_capture(&mut self, board: &Board, mv: Move, fails: &[Move], depth: i32) {
        let stm = board.side_to_move();
        let capture = if mv.is_en_passant() { Piece::Pawn } else { board.piece_on(mv.target()) };
        update::<true>(&mut self.capture[stm][mv.start()][mv.target()][capture], depth);
        for &fail in fails {
            let capture = if fail.is_en_passant() { Piece::Pawn } else { board.piece_on(fail.target()) };
            update::<false>(&mut self.capture[stm][fail.start()][fail.target()][capture], depth);
        }
    }

    pub fn update_main(&mut self, stm: Color, mv: Move, fails: &[Move], depth: i32) {
        update::<true>(&mut self.main[stm][mv.start()][mv.target()], depth);
        for &fail in fails {
            update::<false>(&mut self.main[stm][fail.start()][fail.target()], depth);
        }
    }

    pub fn update_continuation(&mut self, board: &Board, current: Move, fails: &[Move], depth: i32) {
        let piece = board.piece_on(current.start());

        for (kind, ply) in [1, 2].into_iter().enumerate() {
            let previous = board.tail_move(ply);
            if previous == FullMove::NULL {
                continue;
            }

            update::<true>(self.get_continuation_mut(kind, previous, piece, current), depth);
            for &fail in fails {
                let piece = board.piece_on(fail.start());
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

impl Default for History {
    fn default() -> Self {
        Self {
            capture: zeroed_box(),
            main: zeroed_box(),
            continuations: zeroed_box(),
        }
    }
}

/// Returns the bonus for a move based on the depth of the search.
fn bonus(depth: i32) -> i32 {
    130 * depth.min(14) - 30
}

/// Returns the malus for a move based on the depth of the search.
fn malus(depth: i32) -> i32 {
    180 * depth.min(9) + 20
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

fn zeroed_box<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::<T>::from_raw(ptr.cast())
    }
}
