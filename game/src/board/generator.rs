use crate::{
    core::{Bitboard, MoveKind, MoveList, Piece, Square},
    lookup::*,
};

use super::Board;

pub(crate) struct Generator;

impl Generator {
    /// Generates pseudo legal moves for the current state of the board.
    pub fn generate_moves(board: &Board) -> MoveList {
        InnerGenerator::new(board).generate()
    }
}

struct InnerGenerator<'a> {
    board: &'a Board,
    all: Bitboard,
    us: Bitboard,
    them: Bitboard,
    list: MoveList,
}

impl<'a> InnerGenerator<'a> {
    fn new(board: &'a Board) -> Self {
        Self {
            board,
            us: board.us(),
            them: board.them(),
            all: board.us() | board.them(),
            list: MoveList::new(),
        }
    }

    fn generate(mut self) -> MoveList {
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
            let targets = gen(start) & !self.us;

            self.add_captures(start, targets & self.them);
            self.add_quiets(start, targets & !self.them);
        }
    }

    #[inline(always)]
    fn add_captures(&mut self, start: Square, mut targets: Bitboard) {
        while let Some(target) = targets.pop() {
            self.list.add(start, target, MoveKind::Capture)
        }
    }

    #[inline(always)]
    fn add_quiets(&mut self, start: Square, mut targets: Bitboard) {
        while let Some(target) = targets.pop() {
            self.list.add(start, target, MoveKind::Quiet)
        }
    }
}
