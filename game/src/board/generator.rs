use crate::{
    core::{bitboard::Bitboard, moves::Move, piece::Piece, square::Square},
    lookup::*,
};

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
    all: Bitboard,
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
            all: board.us() | board.them(),
            list: Vec::with_capacity(32),
        }
    }

    fn generate(mut self) -> Vec<Move> {
        let occupancies = self.all;

        self.collect_for(Piece::King, king_attacks);
        self.collect_for(Piece::Knight, knight_attacks);

        self.collect_for(Piece::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for(Piece::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for(Piece::Queen, |square| queen_attacks(square, occupancies));

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
            self.list.push(Move::new(start, target, capture));
        }
    }
}
