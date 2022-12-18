use crate::{
    core::{Bitboard, Color, MoveKind, MoveList, Piece, Square},
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

        self.collect_pawn_moves();

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

    fn collect_pawn_moves(&mut self) {
        let bb = self.board.our(Piece::Pawn);

        let (rank_2, rank_7, offset) = match self.board.turn {
            Color::White => (Bitboard::RANK_2, Bitboard::RANK_7, 8i8),
            Color::Black => (Bitboard::RANK_7, Bitboard::RANK_2, -8i8),
        };

        self.collect_double_pushes(rank_2 & bb, offset);
        self.collect_promotions(rank_7 & bb, offset);
        self.collect_regular_pawn_moves(!rank_7 & bb, offset);
    }

    #[inline(always)]
    fn collect_regular_pawn_moves(&mut self, mut bb: Bitboard, offset: i8) {
        while let Some(start) = bb.pop() {
            // Captures
            let targets = pawn_attacks(start, self.board.turn) & self.them;
            self.add_captures(start, targets);

            // One square pawn push
            let target = start.shift(offset);
            if !self.all.contains(target) {
                self.list.add(start, target, MoveKind::Quiet);
            }
        }
    }

    #[inline(always)]
    fn collect_promotions(&mut self, mut bb: Bitboard, offset: i8) {
        while let Some(start) = bb.pop() {
            // Promotion with a capture
            let mut targets = pawn_attacks(start, self.board.turn) & self.them;
            while let Some(target) = targets.pop() {
                self.add_promotion_captures(start, target);
            }

            // Push promotion
            let target = start.shift(offset);
            if !self.all.contains(target) {
                self.add_promotions(start, target);
            }
        }
    }

    #[inline(always)]
    fn collect_double_pushes(&mut self, mut bb: Bitboard, offset: i8) {
        while let Some(start) = bb.pop() {
            let one_up = start.shift(offset);
            let two_up = one_up.shift(offset);

            if !self.all.contains(one_up) & !self.all.contains(two_up) {
                self.list.add(start, two_up, MoveKind::DoublePush);
            }
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

    #[inline(always)]
    fn add_promotions(&mut self, start: Square, target: Square) {
        self.list.add(start, target, MoveKind::PromotionN);
        self.list.add(start, target, MoveKind::PromotionB);
        self.list.add(start, target, MoveKind::PromotionR);
        self.list.add(start, target, MoveKind::PromotionQ);
    }

    #[inline(always)]
    fn add_promotion_captures(&mut self, start: Square, target: Square) {
        self.list.add(start, target, MoveKind::PromotionCaptureN);
        self.list.add(start, target, MoveKind::PromotionCaptureB);
        self.list.add(start, target, MoveKind::PromotionCaptureR);
        self.list.add(start, target, MoveKind::PromotionCaptureQ);
    }
}
