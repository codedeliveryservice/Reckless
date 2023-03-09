use crate::{
    core::{Bitboard, CastlingKind, Color, Move, MoveKind, MoveList, Piece, Square},
    lookup::*,
};

use super::{Board, State};

pub(crate) struct Generator;

impl Generator {
    /// Generates pseudo legal moves for the current state of the board.
    pub fn generate_moves(board: &Board) -> MoveList {
        InnerGenerator::new(board).generate()
    }
}

struct InnerGenerator<'a> {
    board: &'a Board,
    state: &'a State,
    turn: Color,
    all: Bitboard,
    us: Bitboard,
    them: Bitboard,
    list: MoveList,
}

impl<'a> InnerGenerator<'a> {
    fn new(board: &'a Board) -> Self {
        Self {
            board,
            state: &board.state,
            turn: board.turn,
            all: board.us() | board.them(),
            us: board.us(),
            them: board.them(),
            list: MoveList::new(),
        }
    }

    fn generate(mut self) -> MoveList {
        let occupancies = self.all;

        self.collect_pawn_moves();

        self.collect_for(Piece::Knight, knight_attacks);
        self.collect_for(Piece::Bishop, |square| bishop_attacks(square, occupancies));
        self.collect_for(Piece::Rook, |square| rook_attacks(square, occupancies));
        self.collect_for(Piece::Queen, |square| queen_attacks(square, occupancies));

        self.collect_castling();
        self.collect_for(Piece::King, king_attacks);

        self.list
    }

    fn collect_for<T: Fn(Square) -> Bitboard>(&mut self, piece: Piece, gen: T) {
        for start in self.board.our(piece) {
            let targets = gen(start) & !self.us;

            self.add_many(start, targets & self.them, MoveKind::Capture);
            self.add_many(start, targets & !self.them, MoveKind::Quiet);
        }
    }

    fn collect_castling(&mut self) {
        match self.turn {
            Color::White => self.collect_white_castling(),
            Color::Black => self.collect_black_castling(),
        }
    }

    fn collect_white_castling(&mut self) {
        if self.state.castling.is_allowed(CastlingKind::WhiteShort)
            && (self.all & Bitboard::F1_G1).is_empty()
            && !self.board.is_under_attack(Square::E1)
            && !self.board.is_under_attack(Square::F1)
        {
            self.list.push(Move::WHITE_SHORT_CASTLING);
        }

        if self.state.castling.is_allowed(CastlingKind::WhiteLong)
            && (self.all & Bitboard::B1_C1_D1).is_empty()
            && !self.board.is_under_attack(Square::E1)
            && !self.board.is_under_attack(Square::D1)
        {
            self.list.push(Move::WHITE_LONG_CASTLING);
        }
    }

    fn collect_black_castling(&mut self) {
        if self.state.castling.is_allowed(CastlingKind::BlackShort)
            && (self.all & Bitboard::F8_G8).is_empty()
            && !self.board.is_under_attack(Square::E8)
            && !self.board.is_under_attack(Square::F8)
        {
            self.list.push(Move::BLACK_SHORT_CASTLING);
        }

        if self.state.castling.is_allowed(CastlingKind::BlackLong)
            && (self.all & Bitboard::B8_C8_D8).is_empty()
            && !self.board.is_under_attack(Square::E8)
            && !self.board.is_under_attack(Square::D8)
        {
            self.list.push(Move::BLACK_LONG_CASTLING);
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
    fn collect_regular_pawn_moves(&mut self, bb: Bitboard) {
        let offset = self.turn.offset();
        for start in bb {
            // Captures
            let targets = pawn_attacks(start, self.turn) & self.them;
            self.add_many(start, targets, MoveKind::Capture);

            // One square pawn push
            let target = start.shift(offset);
            if !self.all.contains(target) {
                self.list.add(start, target, MoveKind::Quiet);
            }
        }
    }

    #[inline(always)]
    fn collect_promotions(&mut self, bb: Bitboard) {
        let offset = self.turn.offset();
        for start in bb {
            // Promotion with a capture
            for target in pawn_attacks(start, self.turn) & self.them {
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
    fn collect_double_pushes(&mut self, bb: Bitboard) {
        let offset = self.turn.offset();
        for start in bb {
            let one_up = start.shift(offset);
            let two_up = one_up.shift(offset);

            if !self.all.contains(one_up) & !self.all.contains(two_up) {
                self.list.add(start, two_up, MoveKind::DoublePush);
            }
        }
    }

    #[inline(always)]
    fn collect_en_passant_moves(&mut self, bb: Bitboard) {
        if let Some(en_passant) = self.state.en_passant {
            for start in pawn_attacks(en_passant, self.turn.opposite()) & bb {
                self.list.add(start, en_passant, MoveKind::EnPassant);
            }
        }
    }

    #[inline(always)]
    fn add_many(&mut self, start: Square, targets: Bitboard, move_kind: MoveKind) {
        for target in targets {
            self.list.add(start, target, move_kind);
        }
    }

    #[inline(always)]
    fn add_promotions(&mut self, start: Square, target: Square) {
        self.list.add(start, target, MoveKind::PromotionQ);
        self.list.add(start, target, MoveKind::PromotionR);
        self.list.add(start, target, MoveKind::PromotionB);
        self.list.add(start, target, MoveKind::PromotionN);
    }

    #[inline(always)]
    fn add_promotion_captures(&mut self, start: Square, target: Square) {
        self.list.add(start, target, MoveKind::PromotionCaptureQ);
        self.list.add(start, target, MoveKind::PromotionCaptureR);
        self.list.add(start, target, MoveKind::PromotionCaptureB);
        self.list.add(start, target, MoveKind::PromotionCaptureN);
    }
}
