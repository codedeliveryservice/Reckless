use crate::types::{Color, FullMove, Move, Piece, Square};

pub struct CounterMoves {
    table: [[[Move; Square::NUM]; Piece::NUM]; Color::NUM],
}

impl CounterMoves {
    pub fn get(&self, stm: Color, previous: FullMove) -> Option<Move> {
        if previous != FullMove::NULL {
            Some(self.table[!stm][previous.piece()][previous.target()])
        } else {
            None
        }
    }

    pub fn update(&mut self, stm: Color, previous: FullMove, counter: Move) {
        if previous != FullMove::NULL {
            self.table[!stm][previous.piece()][previous.target()] = counter;
        }
    }
}

impl Default for CounterMoves {
    fn default() -> Self {
        CounterMoves {
            table: [[[Move::NULL; Square::NUM]; Piece::NUM]; Color::NUM],
        }
    }
}
