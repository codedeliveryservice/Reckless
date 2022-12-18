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
    turn: Color,
    turn_opposite: Color,
    all: Bitboard,
    us: Bitboard,
    them: Bitboard,
    list: MoveList,
}

impl<'a> InnerGenerator<'a> {
    fn new(board: &'a Board) -> Self {
        Self {
            board,
            turn: board.turn,
            turn_opposite: board.turn.opposite(),
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

        let (starting_rank, promotion_rank) = match self.turn {
            Color::White => (Bitboard::RANK_2, Bitboard::RANK_7),
            Color::Black => (Bitboard::RANK_7, Bitboard::RANK_2),
        };

        self.collect_double_pushes(starting_rank & bb);
        self.collect_promotions(promotion_rank & bb);
        self.collect_regular_pawn_moves(!promotion_rank & bb);
        self.collect_en_passant_moves(bb);
    }

    #[inline(always)]
    fn collect_regular_pawn_moves(&mut self, mut bb: Bitboard) {
        let offset = self.turn.offset();
        while let Some(start) = bb.pop() {
            // Captures
            let targets = pawn_attacks(start, self.turn) & self.them;
            self.add_captures(start, targets);

            // One square pawn push
            let target = start.shift(offset);
            if !self.all.contains(target) {
                self.list.add(start, target, MoveKind::Quiet);
            }
        }
    }

    #[inline(always)]
    fn collect_promotions(&mut self, mut bb: Bitboard) {
        let offset = self.turn.offset();
        while let Some(start) = bb.pop() {
            // Promotion with a capture
            let mut targets = pawn_attacks(start, self.turn) & self.them;
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
    fn collect_double_pushes(&mut self, mut bb: Bitboard) {
        let offset = self.turn.offset();
        while let Some(start) = bb.pop() {
            let one_up = start.shift(offset);
            let two_up = one_up.shift(offset);

            if !self.all.contains(one_up) & !self.all.contains(two_up) {
                self.list.add(start, two_up, MoveKind::DoublePush);
            }
        }
    }

    #[inline(always)]
    fn collect_en_passant_moves(&mut self, bb: Bitboard) {
        let Some(en_passant) = self.board.state.en_passant else { return };
        
        let mut starts = pawn_attacks(en_passant, self.turn.opposite()) & bb;
        while let Some(start) = starts.pop() {
            self.list.add(start, en_passant, MoveKind::EnPassant);
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
