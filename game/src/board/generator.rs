use crate::core::{bitboard::Bitboard, moves::Move, piece::Piece, square::Square};

use super::Board;

pub struct Generator;

impl Generator {
    /// Generates pseudo legal moves for the current state of the board.
    pub fn generate_moves(board: &Board) -> Vec<Move> {
        InnerGenerator::new(board).generate()
    }
}

struct InnerGenerator<'a> {
    board: &'a Board,
    us: Bitboard,
    them: Bitboard,
    list: Vec<Move>,
}

impl<'a> InnerGenerator<'a> {
    fn new(board: &'a Board) -> Self {
        Self {
            board,
            us: board.us(),
            them: board.them(),
            list: Vec::with_capacity(32),
        }
    }

    fn generate(mut self) -> Vec<Move> {
        use crate::lookup::*;

        self.collect_for(Piece::King, king_attacks);

        self.list
    }

    fn collect_for<T: Fn(Square) -> Bitboard>(&mut self, piece: Piece, gen: T) {
        let mut bb = self.board.our(piece);
        while let Some(start) = bb.pop() {
            self.add_moves(start, gen(start) & !self.us);
        }
    }

    fn add_moves(&mut self, start: Square, mut targets: Bitboard) {
        while let Some(target) = targets.pop() {
            let capture = self.them.contains(target);

            match capture {
                true => self.list.push(Move::capture(start, target)),
                false => self.list.push(Move::quiet(start, target)),
            };
        }
    }
}
