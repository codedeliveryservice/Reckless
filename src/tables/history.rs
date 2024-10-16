use crate::{
    board::Board,
    parameters::*,
    types::{Color, FullMove, Move, Piece, Square},
};

const MAX_HISTORY: i32 = 16384;

type Butterfly<T> = [[T; Square::NUM]; Square::NUM];
type PieceSquare<T> = [[T; Square::NUM]; Piece::NUM + 1];

/// The history heuristic is a table that keep track of how successful a move has been in the past.
/// The idea is that if a move has been successful in the past, it's likely to be successful in the
/// future as well.
///
/// See [History Heuristic](https://www.chessprogramming.org/History_Heuristic) for more information.
#[derive(Clone)]
pub struct History {
    main: Box<[Butterfly<i32>; Color::NUM]>,
    counter: Box<PieceSquare<PieceSquare<i32>>>,
    followup: Box<PieceSquare<PieceSquare<i32>>>,
    capture: Box<[Butterfly<[i32; Piece::NUM]>; Color::NUM]>,
}

impl History {
    pub fn get_capture(&self, stm: Color, mv: Move, capture: Piece) -> i32 {
        self.capture[stm][mv.start()][mv.target()][capture]
    }

    pub fn get_main(&self, stm: Color, mv: Move) -> i32 {
        self.main[stm][mv.start()][mv.target()]
    }

    pub fn get_counter(&self, continuation: FullMove, piece: Piece, current: Move) -> i32 {
        self.counter[continuation.piece()][continuation.target()][piece][current.target()]
    }

    pub fn get_followup(&self, continuation: FullMove, piece: Piece, current: Move) -> i32 {
        self.followup[continuation.piece()][continuation.target()][piece][current.target()]
    }

    pub fn update_capture(&mut self, board: &Board, mv: Move, fails: &[Move], depth: i32) {
        let stm = board.side_to_move();
        let capture = if mv.is_en_passant() { Piece::Pawn } else { board.piece_on(mv.target()) };

        increase(&mut self.capture[stm][mv.start()][mv.target()][capture], depth);
        for &fail in fails {
            let capture = if fail.is_en_passant() { Piece::Pawn } else { board.piece_on(fail.target()) };
            decrease(&mut self.capture[stm][fail.start()][fail.target()][capture], depth);
        }
    }

    pub fn update_main(&mut self, stm: Color, mv: Move, fails: &[Move], depth: i32) {
        increase(&mut self.main[stm][mv.start()][mv.target()], depth);
        for &fail in fails {
            decrease(&mut self.main[stm][fail.start()][fail.target()], depth);
        }
    }

    pub fn update_continuation(&mut self, board: &Board, current: Move, fails: &[Move], depth: i32) {
        let piece = board.piece_on(current.start());

        macro_rules! update_history {
            ($table:expr, ply: $ply:expr) => {
                let prev = board.tail_move($ply);
                if prev != FullMove::NULL {
                    increase(&mut $table[prev.piece()][prev.target()][piece][current.target()], depth);

                    for &fail in fails {
                        let piece = board.piece_on(fail.start());
                        decrease(&mut $table[prev.piece()][prev.target()][piece][fail.target()], depth);
                    }
                }
            };
        }
        update_history!(self.counter, ply: 1);
        update_history!(self.followup, ply: 2);
    }
}

impl Default for History {
    fn default() -> Self {
        Self {
            main: zeroed_box(),
            counter: zeroed_box(),
            followup: zeroed_box(),
            capture: zeroed_box(),
        }
    }
}

fn bonus(depth: i32) -> i32 {
    (history_bonus() * depth + history_bonus_base()).min(history_bonus_max())
}

fn malus(depth: i32) -> i32 {
    (history_malus() * depth + history_malus_base()).min(history_malus_max())
}

fn increase(v: &mut i32, depth: i32) {
    let bonus = bonus(depth);
    *v += bonus - bonus * *v / MAX_HISTORY;
}

fn decrease(v: &mut i32, depth: i32) {
    let malus = malus(depth);
    *v -= malus + malus * *v / MAX_HISTORY;
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
