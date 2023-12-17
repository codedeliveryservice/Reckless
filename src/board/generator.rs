use super::{Board, InternalState};
use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, queen_attacks, rook_attacks},
    types::{Bitboard, CastlingKind, Color, Move, MoveKind, MoveList, Piece, Rank, Square},
};

pub struct Generator<'a> {
    board: &'a Board,
    state: &'a InternalState,
    stm: Color,
    all: Bitboard,
    us: Bitboard,
    them: Bitboard,
    list: MoveList,
}

impl<'a> Generator<'a> {
    pub fn new(board: &'a Board) -> Self {
        Self {
            board,
            state: &board.state,
            stm: board.side_to_move,
            all: board.occupancies(),
            us: board.us(),
            them: board.them(),
            list: MoveList::new(),
        }
    }

    /// Generates pseudo legal moves for the current state of the board.
    pub fn generate(mut self) -> MoveList {
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

    /// Adds move for the piece type using the specified move generator function.
    fn collect_for<T: Fn(Square) -> Bitboard>(&mut self, piece: Piece, gen: T) {
        for start in self.board.our(piece) {
            let targets = gen(start) & !self.us;

            self.add_many(start, targets & self.them, MoveKind::Capture);
            self.add_many(start, targets & !self.them, MoveKind::Quiet);
        }
    }

    fn collect_castling(&mut self) {
        use crate::types::{BlackKingSide, BlackQueenSide, WhiteKingSide, WhiteQueenSide};

        match self.stm {
            Color::White => {
                self.collect_castling_kind::<WhiteKingSide>();
                self.collect_castling_kind::<WhiteQueenSide>();
            }
            Color::Black => {
                self.collect_castling_kind::<BlackKingSide>();
                self.collect_castling_kind::<BlackQueenSide>();
            }
        }
    }

    /// Adds the castling move to the move list if it's allowed.
    ///
    /// This method does not check if the king is in check after the castling,
    /// as this will be checked by the `make_move` method.
    fn collect_castling_kind<KIND: CastlingKind>(&mut self) {
        if (KIND::PATH_MASK & self.all).is_empty() && self.state.castling.is_allowed::<KIND>() {
            for square in KIND::CHECK_SQUARES {
                if self.board.is_under_attack(square) {
                    return;
                }
            }

            self.list.push(KIND::CASTLING_MOVE);
        }
    }

    /// Adds all pawn moves to the move list.
    fn collect_pawn_moves(&mut self) {
        let pawns = self.board.our(Piece::Pawn);
        let seventh_rank = match self.stm {
            Color::White => Bitboard::rank(Rank::R7),
            Color::Black => Bitboard::rank(Rank::R2),
        };

        self.collect_pawn_pushes(pawns, seventh_rank);
        self.collect_pawn_captures(pawns, seventh_rank);
        self.collect_en_passant_moves(pawns);
    }

    /// Adds single, double and promotion pawn pushes to the move list.
    fn collect_pawn_pushes(&mut self, pawns: Bitboard, seventh_rank: Bitboard) {
        let (up, third_rank) = match self.stm {
            Color::White => (8, Bitboard::rank(Rank::R3)),
            Color::Black => (-8, Bitboard::rank(Rank::R6)),
        };

        let empty = !self.all;

        let non_promotions = pawns & !seventh_rank;
        let single_pushes = non_promotions.shift(up) & empty;
        let double_pushes = (single_pushes & third_rank).shift(up) & empty;

        for target in single_pushes {
            self.add(target.shift(-up), target, MoveKind::Quiet);
        }

        for target in double_pushes {
            self.add(target.shift(-up * 2), target, MoveKind::DoublePush);
        }

        let promotions = (pawns & seventh_rank).shift(up) & empty;
        for target in promotions {
            let start = target.shift(-up);
            self.add(start, target, MoveKind::PromotionQ);
            self.add(start, target, MoveKind::PromotionR);
            self.add(start, target, MoveKind::PromotionB);
            self.add(start, target, MoveKind::PromotionN);
        }
    }

    /// Adds regular pawn captures and promotion captures to the move list.
    fn collect_pawn_captures(&mut self, pawns: Bitboard, seventh_rank: Bitboard) {
        let promotions = pawns & seventh_rank;
        for start in promotions {
            let captures = self.them & pawn_attacks(start, self.stm);
            for target in captures {
                self.add(start, target, MoveKind::PromotionCaptureQ);
                self.add(start, target, MoveKind::PromotionCaptureR);
                self.add(start, target, MoveKind::PromotionCaptureB);
                self.add(start, target, MoveKind::PromotionCaptureN);
            }
        }

        let non_promotions = pawns & !seventh_rank;
        for start in non_promotions {
            let targets = self.them & pawn_attacks(start, self.stm);
            self.add_many(start, targets, MoveKind::Capture);
        }
    }

    fn collect_en_passant_moves(&mut self, pawns: Bitboard) {
        if self.state.en_passant != Square::None {
            let pawns = pawns & pawn_attacks(self.state.en_passant, !self.stm);
            for pawn in pawns {
                self.add(pawn, self.state.en_passant, MoveKind::EnPassant);
            }
        }
    }

    fn add(&mut self, start: Square, target: Square, move_kind: MoveKind) {
        self.list.push(Move::new(start, target, move_kind));
    }

    /// Adds all possible moves from the given starting square to the squares of the `targets` bitboard.
    fn add_many(&mut self, start: Square, targets: Bitboard, move_kind: MoveKind) {
        for target in targets {
            self.add(start, target, move_kind);
        }
    }
}
